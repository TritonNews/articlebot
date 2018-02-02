use std::time::Duration;
use std::thread;
use reqwest::Client;
use reqwest::header::UserAgent;
use reqwest::Result;
use slack::Sender;
use mongodb::db::{Database, ThreadedDatabase};
use chrono::prelude::*;
use serde_json::Value;

const USER_AGENT: &'static str =
  "Mozilla/5.0 (Windows NT 5.1; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/46.0.2486.0 Safari/537.36 Edge/13.10586";

const UPDATE_INTERVAL_SECONDS: u64 = 600;

#[allow(dead_code)]
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Member {
    id: String,
    avatar_hash: String,
    full_name: String,
    initials: String,
    username: String
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Action {
    id: String,
    data: Value,
    date: String, // TODO: Switch to using DateTime<Utc> later
    #[serde(rename = "type")]
    action_type: String,
    #[serde(rename = "idMemberCreator")]
    creator_id: String,
    #[serde(rename = "memberCreator")]
    creator: Member
}

pub struct BoardHandler<'a> {
    pub id: String,
    db: &'a Database,
    sender: &'a Sender,
    http_url: String,
    http_since_parameter: DateTime<Utc>,
    http_token_parameters: String,
    http_client: Client
}

impl<'a> BoardHandler<'a> {
    pub fn new(board_id: &str, trello_api_key: &str, trello_oauth_token: &str, mongodb: &'a Database, slack_sender: &'a Sender) -> BoardHandler<'a> {
        BoardHandler {
            id: board_id.to_string(),
            db: mongodb,
            sender: slack_sender,
            http_since_parameter: Utc::now(),
            http_url: format!("https://api.trello.com/1/boards/{}", board_id).to_string(),
            http_token_parameters: format!("key={}&token={}", trello_api_key, trello_oauth_token).to_string(),
            http_client: Client::new()
        }
    }

    pub fn listen(&mut self) -> Result<()> {
        loop {
            let mut resp = self.http_client
                .get(&format!("{}/actions?filter=updateCard&since={}&{}", self.http_url, self.http_since_parameter, self.http_token_parameters))
                .header(UserAgent::new(USER_AGENT.to_string()))
                .send()?;

            let actions : Vec<Action> = resp.json()?;

            for action in actions {
                // Make sure that we only capture when a card is moved between lists
                if let Some(list_before) = action.data.get("listBefore") {
                    let list_after = action.data.get("listAfter").unwrap();
                    let card = action.data.get("card").unwrap();

                    let list_before_name = list_before.get("name").unwrap().as_str().unwrap();
                    let list_after_name = list_after.get("name").unwrap().as_str().unwrap();
                    let card_title = card.get("name").unwrap().as_str().unwrap();

                    let trello_coll = self.db.collection("trello");
                    let trello_lookup = doc! {
                        "name": &action.creator.full_name
                    };

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

            self.http_since_parameter = Utc::now();

            thread::sleep(Duration::from_secs(UPDATE_INTERVAL_SECONDS));
        }
    }
}