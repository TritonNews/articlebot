extern crate slack;
use slack::{Event, EventHandler, RtmClient};

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
    let api_key = std::env::var("SLACK_API_KEY");
    match api_key {
      Ok(key) => {
        let mut handler = SlackArticleHandler;
        let client = RtmClient::login_and_run(&key, &mut handler);
        match client {
            Ok(_) => {}
            Err(err) => panic!("Error: {}", err),
        }
      }
      Err(err) => {
        panic!("Error: {}", err)
      }
    }
}
