extern crate slack;
extern crate slack_api;
extern crate slack_hook;
extern crate reqwest;
extern crate chrono;
#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
#[macro_use(bson, doc)]
extern crate bson;
extern crate mongodb;

mod trello;
mod trello_models;
mod trello_listeners;
mod commands;

use std::{env, thread};
use std::time::Duration;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::{Mutex, Arc};

use trello::{BoardHandler, CardHandler};
use trello_listeners::RelayActionListener;

use commands::CommandHandler;

use slack::{Event, EventHandler, RtmClient, Message};
use slack_hook::{Slack, PayloadBuilder};
use mongodb::{Client, ThreadedClient};
use mongodb::db::Database;

const MONGODB_HOSTNAME: &'static str = "localhost";
const MONGODB_PORT: u16 = 27017;
const MONGODB_DATABASE: &'static str = "articlebot";
const TRY_FLUSH_INTERVAL: u64 = 30;

struct SlackHandler {
    command_handler: CommandHandler,
    buffer_rx: Receiver<String>
}

impl SlackHandler {
    fn new(command_handler: CommandHandler, buffer_rx: Receiver<String>) -> SlackHandler {
        SlackHandler {
            command_handler: command_handler,
            buffer_rx: buffer_rx
        }
    }
}

impl EventHandler for SlackHandler {
    fn on_event(&mut self, cli: &RtmClient, event: Event) {
        let sender = cli.sender();
        if let Event::Message(boxed_message) = event {
            if let Message::Standard(message) = *boxed_message {
                self.command_handler.handle_message(message, cli).expect("Slack commands error");
            }
            else if let Message::BotMessage(_) = *boxed_message {
                loop {
                    match self.buffer_rx.try_recv() {
                        Ok(channel_message) => {
                            let split_channel_message: Vec<&str> = channel_message.split("|").collect();
                            sender.send_message(split_channel_message[0], split_channel_message[1]).expect("Slack sender error");
                        }
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => panic!("Board listener detached!")
                    }
                }
            }
        }
    }

    fn on_connect(&mut self, _cli: &RtmClient) {
        info!("v{} connected.", env::var("CARGO_PKG_VERSION").unwrap());
    }

    fn on_close(&mut self, _cli: &RtmClient) {
        info!("v{} disconnected.", env::var("CARGO_PKG_VERSION").unwrap());
    }
}

fn open_database_connection() -> Database {
    return Client::connect(MONGODB_HOSTNAME, MONGODB_PORT).expect("MongoDB connection error").db(MONGODB_DATABASE);
}

fn main() {
    // Logging utilities
    env_logger::init();

    // Get all environment variables
    let slack_api_key = env::var("SLACK_API_KEY").expect("Slack API key not found");
    let slack_webhook = env::var("SLACK_WEBHOOK").expect("Slack webhook not found");
    let trello_api_key = env::var("TRELLO_API_KEY").expect("Trello API key not found");
    let trello_oauth_token = env::var("TRELLO_OAUTH_TOKEN").expect("Trello OAuth token not found");
    let trello_board_id = env::var("TRELLO_BOARD_ID").expect("Trello board ID not found");

    let (tx, rx) = mpsc::channel();
    let buffer_count = Arc::new(Mutex::new(0));

    // Slack webhook occasionally sends messages to the #articlebot-reserved to flush message buffer
    let webhook_buffer_count = Arc::clone(&buffer_count);
    thread::spawn(move || {
        let slack = Slack::new(&slack_webhook[..]).unwrap();
        loop {
            // This inner scope is necessary because it forces the Mutex to unlock before the thread sleeps
            {
                let mut message_count = webhook_buffer_count.lock().unwrap();
                if *message_count > 0 {
                    info!("Flushing {} messages in buffer ...", *message_count);

                    let payload = PayloadBuilder::new()
                      .text(&format!("articlebot is now flushing {} messages in its internal mpsc channel.", *message_count)[..])
                      .build()
                      .unwrap();

                    slack.send(&payload).expect("Webhook flush error");

                    *message_count = 0;
                }
            }
            thread::sleep(Duration::from_secs(TRY_FLUSH_INTERVAL));
        }
    });

    // Offload the Trello updater to its own thread so it doesn't block the main thread
    let trello_buffer_count = Arc::clone(&buffer_count);
    thread::spawn(move || {
        let card_handler = CardHandler::new(&trello_api_key, &trello_oauth_token);
        let action_listener = RelayActionListener::new(open_database_connection(), card_handler, tx, trello_buffer_count);
        let mut board_handler = BoardHandler::new(&trello_board_id, &trello_api_key, &trello_oauth_token, action_listener);
        board_handler.listen().expect("Trello handler error");
    });

    // Slack event handler
    let command_handler = CommandHandler::new(open_database_connection());
    let mut slack_handler = SlackHandler::new(command_handler, rx);
    RtmClient::login_and_run(&slack_api_key, &mut slack_handler).expect("Slack client error");
}
