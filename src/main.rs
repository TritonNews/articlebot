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

mod trello;
mod trello_models;

use std::{env, thread};
use trello::{BoardHandler, BoardListener, get_card, get_card_members};
use trello_models::Action;
use slack::{Event, EventHandler, RtmClient, Message, Sender};
use reqwest::Client as ReqwestClient;
use mongodb::{Client, ThreadedClient};
use mongodb::db::{Database, ThreadedDatabase};
use bson::Bson;

const MONGODB_HOSTNAME: &'static str = "localhost";
const MONGODB_PORT: u16 = 27017;
const MONGODB_DATABASE: &'static str = "articlebot";

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

struct SlackBoardListener {
    db: Database,
    sender: Sender,
    http_token_parameters: String,
    http_client: ReqwestClient
}

impl BoardListener for SlackBoardListener {
    fn get_filtered_actions(&self) -> &str {
        return &"updateCard";
    }

    fn on_action(&self, action : &Action) {
        // Make sure that we only capture when a card is moved between lists
        if let Some(list_before) = action.data.get("listBefore") {
            let card = get_card(action.data.get("card").unwrap().get("id").unwrap().as_str().unwrap(),
                &self.http_token_parameters[..], &self.http_client).expect("Trello card error");
            let card_members = get_card_members(&card, &self.http_token_parameters[..], &self.http_client).expect("Trello card error");
            let card_title = card.name;

            let list_before_name = list_before.get("name").unwrap().as_str().unwrap();
            let list_after = action.data.get("listAfter").unwrap();
            let list_after_name = list_after.get("name").unwrap().as_str().unwrap();

            info!("Card \"{}\" was moved from \"{}\" to \"{}\".", card_title, list_before_name, list_after_name);

            for member in card_members {
                let trello_coll = self.db.collection("trello");
                let trello_lookup = doc! {
                    "name": &member.full_name
                };

                info!("Member \"{}\" is associated with this card.", &member.full_name);

                // If any Slack user is tracking this Trello user, find all Slack DM channel IDs through MongoDB and send a message to each one
                if let Some(tdoc) = trello_coll.find_one(Some(trello_lookup), None).expect("Failed to find document") {
                    let trackers = tdoc.get_array("trackers").unwrap();

                    let slack_coll = self.db.collection("slack");

                    for tracker in trackers {
                      let slack_lookup = doc! {
                        "uid": tracker.as_str().unwrap()
                      };
                      // Tracker refers to a slack user that must exist
                      let sdoc = slack_coll.find_one(Some(slack_lookup), None).expect("Failed to find document").unwrap();
                      let channel = sdoc.get_str("cid").unwrap();

                      self.sender.send_message(channel, &format!("Your card \"{}\" has been moved from \"{}\" to \"{}\".", card_title, list_before_name, list_after_name))
                        .expect("Slack sender error");
                    }
                }
            }
        }
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
    let trello_api_key = env::var("TRELLO_API_KEY").expect("Trello API key not found");
    let trello_oauth_token = env::var("TRELLO_OAUTH_TOKEN").expect("Trello OAuth token not found");
    let trello_board_id = env::var("TRELLO_BOARD_ID").expect("Trello board ID not found");

    // Create shared Slack utilities
    let slack_client = RtmClient::login(&slack_api_key).expect("Slack connection error");
    let slack_sender = slack_client.sender().clone();

    // Offload the Slack message receiver/client to its own thread so it doesn't block the main thread
    thread::spawn(move || {

        // Connect to Trello (will block main thread)
        let db = open_database_connection();
        let board_listener = SlackBoardListener {
            db: db,
            sender: slack_sender,
            http_token_parameters: format!("key={}&token={}", &trello_api_key, &trello_oauth_token).to_string(),
            http_client: ReqwestClient::new()
        };
        let mut board_handler = BoardHandler::new(&trello_board_id, &trello_api_key, &trello_oauth_token, board_listener);
        board_handler.listen().expect("Event loop error");
    });

    let db = open_database_connection();
    let mut slack_handler = SlackHandler {
        db: db
    };
    slack_client.run(&mut slack_handler).expect("Slack client error");
}
