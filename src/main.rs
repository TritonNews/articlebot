#[macro_use(bson, doc)]
extern crate bson;
extern crate mongodb;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate slack;
extern crate reqwest;

mod trello;

use std::env;
use trello::Board;
use slack::{Event, EventHandler, RtmClient, Message};
use mongodb::{Client, ThreadedClient};
use mongodb::db::{Database, ThreadedDatabase};
use bson::Bson;

struct SlackHandler<'a> {
  db: &'a Database
}

impl<'a> EventHandler for SlackHandler<'a> {
  fn on_event(&mut self, cli: &RtmClient, event: Event) {
    match event {
      Event::Message(boxed_message) => {
        match *boxed_message {
          Message::Standard(message) => {
            let tracker = message.user.unwrap();
            let tracking = message.text.unwrap();
            let channel = message.channel.unwrap();

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
            if let Some(sdoc) = slack_coll.find_one_and_delete(slack_lookup, None).expect("Failed to delete document") {
              let trello_lookup_old = doc! {
                "tracking": sdoc.get_str("tracking").unwrap()
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
              "cid": &channel,
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

            let sender = cli.sender();
            sender.send_message(&channel[..], &format!("You will now be notified when {}'s articles are moved in Trello.", tracking)[..]).expect("Slack sender error");
          },
          _ => ()
        }
      },
      _ => ()
    }
  }

  fn on_close(&mut self, _cli: &RtmClient) {
    println!("articlebot v{} disconnecting ...", std::env::var("CARGO_PKG_VERSION").unwrap());
  }

  fn on_connect(&mut self, _cli: &RtmClient) {
    println!("articlebot v{} connecting ...", std::env::var("CARGO_PKG_VERSION").unwrap());
  }
}

fn main() {
  // Get all environment variables
  let slack_api_key = env::var("SLACK_API_KEY").expect("Slack API key not found");
  let trello_api_key = env::var("TRELLO_API_KEY").expect("Trello API key not found");
  let trello_oauth_token = env::var("TRELLO_OAUTH_TOKEN").expect("Trello OAuth token not found");
  let trello_board_id = env::var("TRELLO_BOARD_ID").expect("Trello board ID not found");
  let mongodb_host = env::var("MONGODB_HOSTNAME").expect("MongoDB hostname not found");
  let mongodb_port = env::var("MONGODB_PORT").expect("MongoDB port not found");
  let mongodb_user = env::var("MONGODB_USERNAME").expect("MongoDB username not found");
  let mongodb_pass = env::var("MONGODB_PASSWORD").expect("MongoDB password not found");

  // Connect to MongoDB
  let mongo_client = Client::connect(&mongodb_host[..], mongodb_port.parse::<u16>().unwrap()).expect("MongoDB connection error");
  let db = mongo_client.db("articlebot");
  db.auth(&mongodb_user[..], &mongodb_pass[..]).unwrap();

  // Create the Slack handler
  let mut slack_handler = SlackHandler {
    db: &db
  };

  // Connect to Slack, attach the handler, and start listening for events
  let slack_client = RtmClient::login(&slack_api_key).expect("Slack connection error");
  slack_client.run(&mut slack_handler).expect("Slack client error");

  // Connect to Trello
  // TODO: Remove the Trello module's dependencies on Slack and MongoDB
  let mut board = Board::new(&trello_board_id, &trello_api_key, &trello_oauth_token, &db, slack_client.sender());
  board.listen().ok().expect("Something went wrong! The board event loop should block forever.");
}
