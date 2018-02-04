extern crate slack;
extern crate reqwest;
extern crate chrono;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
#[macro_use(bson, doc)]
extern crate bson;
extern crate mongodb;
extern crate slack_hook;

mod trello;
mod trello_models;
mod trello_listeners;

use std::{env, thread};
use std::time::Duration;
use trello::BoardHandler;
use trello_listeners::RelayBoardListener;
use slack::{Event, EventHandler, RtmClient, Message};
use slack_hook::{Slack, PayloadBuilder};
use mongodb::{Client, ThreadedClient};
use mongodb::db::{Database, ThreadedDatabase};
use bson::Bson;

const MONGODB_HOSTNAME: &'static str = "localhost";
const MONGODB_PORT: u16 = 27017;
const MONGODB_DATABASE: &'static str = "articlebot";
const FLUSH_MESSAGES_DELAY_SECONDS: u64 = 30;

struct SlackHandler {
    db: Database
}

impl EventHandler for SlackHandler {
    fn on_event(&mut self, cli: &RtmClient, event: Event) {
        let sender = cli.sender();
        if let Event::Message(boxed_message) = event {
            if let Message::Standard(message) = *boxed_message {
                let text : &str = &message.text.unwrap()[..];
                let channel : &str = &message.channel.unwrap()[..];

                let split_text: Vec<&str> = text.split(" ").collect();
                let command = split_text[0].to_lowercase();
                let args = &split_text[1..];

                let user = message.user.unwrap();

                info!("Message from {}: {}", user, text);
                info!("Interpreting as COMMAND={} ARGUMENTS={:?}", command, args);

                if command == "hello" || command == "hi" {
                    sender.send_message(channel, "Hello there.").expect("Slack sender error");
                }
                else if command == "whoami" {
                    let tracker = user;

                    sender.send_message(channel, "Fetching user information ...").expect("Slack sender error");

                    // Slack collection in MongoDB (key: tracker id, other data: channel id, tracking name)
                    let slack_coll = self.db.collection("slack");
                    let slack_lookup = doc! {
                        "uid": &tracker
                    };

                    if let Some(sdoc) = slack_coll.find_one(Some(slack_lookup), None).expect("Failed to find document") {
                        sender.send_message(channel, &format!("You are {} on Slack. This channel is {}.", tracker, channel)[..]).expect("Slack sender error");
                        sender.send_message(channel, &format!("You are currently tracking \"{}\" on Trello.", sdoc.get_str("tracking").unwrap())[..])
                            .expect("Slack sender error");
                    }
                    else {
                        sender.send_message(channel, &format!("You are {} on Slack. This channel is {}.", tracker, channel)[..]).expect("Slack sender error");
                        sender.send_message(channel, "You are currently not tracking a Trello user.").expect("Slack sender error");
                    }
                }
                else if command == "track" {
                    let tracker = user;
                    let tracking = args.join(" ");

                    // Slack collection in MongoDB (key: tracker id, other data: channel id, tracking name)
                    let slack_coll = self.db.collection("slack");
                    let slack_lookup = doc! {
                        "uid": &tracker
                    };

                    // Trello collection in MongoDB (key: tracking name, other data: list of trackers)
                    let trello_coll = self.db.collection("trello");
                    let trello_lookup = doc! {
                        "name": &tracking
                    };

                    // Delete all previous records in the Slack and Trello collections if they exist
                    if let Some(sdoc) = slack_coll.find_one_and_delete(slack_lookup, None).expect("Failed to find and delete document") {
                        let trello_lookup_old = doc! {
                            "name": sdoc.get_str("tracking").unwrap()
                        };
                        if let Some(tdoc) = trello_coll.find_one(Some(trello_lookup_old.clone()), None).expect("Failed to find document") {
                            // Remove our current tracker from where it was originally tracking
                            let mut trackers_old = tdoc.get_array("trackers").unwrap().clone();
                            let index = trackers_old.iter().position(|tracker_old| *tracker_old.as_str().unwrap() == tracker).unwrap();
                            trackers_old.remove(index);

                            // If the Trello user has no trackers, delete it. Otherwise, update it to reflect the changes made to its changes.
                            if trackers_old.is_empty() {
                                trello_coll.delete_one(trello_lookup_old, None).expect("Failed to delete document");
                            }
                            else {
                                let mut tdoc_new = tdoc.clone();
                                tdoc_new.insert_bson("trackers".to_string(), Bson::Array(trackers_old));
                                trello_coll.update_one(trello_lookup_old, tdoc_new, None).expect("Failed to update document");
                            }
                        }
                    }

                    // Insert a new Slack document specifying the tracker's information
                    slack_coll.insert_one(doc! {
                        "uid": &tracker,
                        "cid": channel,
                        "tracking": &tracking
                    }, None).expect("Failed to insert document");

                    // Update (or create/insert) the Trello document that contains the trackers
                    if let Some(tdoc) = trello_coll.find_one(Some(trello_lookup.clone()), None).expect("Failed to find document") {
                        let mut trackers = tdoc.get_array("trackers").unwrap().clone();
                        trackers.push(Bson::String(tracker));

                        let mut tdoc_new = tdoc.clone();
                        tdoc_new.insert_bson("trackers".to_string(), Bson::Array(trackers));

                        trello_coll.update_one(trello_lookup, tdoc_new, None).expect("Failed to update document");
                    }
                    else {
                        trello_coll.insert_one(doc! {
                            "name": &tracking,
                            "trackers": [&tracker]
                        }, None).expect("Failed to insert document");
                    }

                    sender.send_message(channel, &format!("You will now be notified when {}'s articles are moved in Trello.", tracking)[..])
                        .expect("Slack sender error");
                }
                else {
                    sender.send_message(channel, &format!("I did not understand your command \"{}\".", command)[..])
                        .expect("Slack sender error");
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
    pretty_env_logger::init();

    // Get all environment variables
    let slack_api_key = env::var("SLACK_API_KEY").expect("Slack API key not found");
    let slack_webhook = env::var("SLACK_WEBHOOK").expect("Slack webhook not found");
    let trello_api_key = env::var("TRELLO_API_KEY").expect("Trello API key not found");
    let trello_oauth_token = env::var("TRELLO_OAUTH_TOKEN").expect("Trello OAuth token not found");
    let trello_board_id = env::var("TRELLO_BOARD_ID").expect("Trello board ID not found");

    // Create shared Slack utilities
    let slack_client = RtmClient::login(&slack_api_key).expect("Slack connection error");
    let slack_sender = slack_client.sender().clone();

    // Slack webhook occasionally sends messages to itself to flush message buffer
    thread::spawn(move || {
        loop {
            let slack = Slack::new(&slack_webhook[..]).unwrap();
            let p = PayloadBuilder::new()
              .text("Flushing message buffer ...")
              .channel("@articlebot")
              .username("articlebot")
              .build()
              .unwrap();

            let res = slack.send(&p);
            match res {
                Ok(()) => info!("Successively flushed message buffer by sending message to self"),
                Err(e) => error!("Error flushing message buffer: {:?}", e)
            }

            thread::sleep(Duration::from_secs(FLUSH_MESSAGES_DELAY_SECONDS));
        }
    });

    // Offload the Trello updater to its own thread so it doesn't block the main thread
    thread::spawn(move || {
        // Connect to Trello (will block main thread)
        let db = open_database_connection();
        let board_listener = RelayBoardListener::new(db, slack_sender, &trello_api_key, &trello_oauth_token);
        let mut board_handler = BoardHandler::new(&trello_board_id, &trello_api_key, &trello_oauth_token, board_listener);
        board_handler.listen().expect("Event loop error");
    });

    // Slack event handler
    let db = open_database_connection();
    let mut slack_handler = SlackHandler {
        db: db
    };
    slack_client.run(&mut slack_handler).expect("Slack client error");
}
