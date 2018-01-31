#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate slack;
extern crate reqwest;
extern crate mongodb;

mod trello;

use std::env;
use slack::{Event, EventHandler, RtmClient};
use trello::Board;
use mongodb::{Client, ThreadedClient};
use mongodb::db::{Database, ThreadedDatabase};

struct SlackHandler {
  db: Database
}

impl EventHandler for SlackHandler {
  fn on_event(&mut self, cli: &RtmClient, event: Event) {

  }

  fn on_close(&mut self, cli: &RtmClient) {
    println!("articlebot v{} disconnecting ...", std::env::var("CARGO_PKG_VERSION").unwrap());
  }

  fn on_connect(&mut self, cli: &RtmClient) {
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
    db: db
  };

  // Connect to Slack, attach the handler, and start listening for events
  let slack_client = RtmClient::login(&slack_api_key).expect("Slack connection error");
  slack_client.run(&mut slack_handler);

  // Connect to Trello
  // TODO: Use a generic board handler to keep Trello API separate from Slack
  let mut board = Board::new(&trello_board_id, &trello_api_key, &trello_oauth_token, slack_client.sender());
  board.listen();
}
