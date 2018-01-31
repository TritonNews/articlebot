#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate slack;
extern crate reqwest;

mod trello;

use slack::{Event, EventHandler, RtmClient};
use trello::Board;
use std::env;

struct SlackHandler;

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

    // Create the Slack handler
    let mut handler = SlackHandler;

    // Log into Slack, connect the handler, and start listening for events
    let client = RtmClient::login(&slack_api_key).expect("Slack connection error");
    client.run(&mut handler);

    // Create the Trello board representation and connect it to Slack
    let mut board = Board::new(&trello_board_id, &trello_api_key, &trello_oauth_token, client.sender());
    board.listen();
}
