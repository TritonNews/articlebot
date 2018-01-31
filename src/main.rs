#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate slack;
extern crate reqwest;

mod trello;

use slack::{Event, EventHandler, RtmClient};
use trello::Board;
use std::env;

struct SlackArticleHandler;

impl EventHandler for SlackArticleHandler {
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
    let slack_api_key = env::var("SLACK_API_KEY").expect("Slack API key not found");
    let trello_api_key = env::var("TRELLO_API_KEY").expect("Trello API key not found");
    let trello_oauth_token = env::var("TRELLO_OAUTH_TOKEN").expect("Trello OAuth token not found");
    let trello_board_id = env::var("TRELLO_BOARD_ID").expect("Trello board ID not found");

    let mut board = Board::new(&trello_board_id, &trello_api_key, &trello_oauth_token);
    board.load();
    board.start_tracking();

    let mut handler = SlackArticleHandler;
    let client = RtmClient::login_and_run(&slack_api_key, &mut handler);
    match client {
        Ok(_) => {}
        Err(err) => panic!("Error: {}", err),
    }
}
