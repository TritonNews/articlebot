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

const ACTION_TYPE_FILTERS: &'static str = "updateCard";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Member {
  id: String,
  avatar_hash: String,
  full_name: String,
  initials: String,
  username: String
}

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

pub struct Board<'a> {
  pub id: String,
  pub name: String,
  db: &'a Database,
  sender: &'a Sender,
  http_url: String,
  http_since_parameter: DateTime<Utc>,
  http_token_parameters: String,
  http_client: Client
}

impl<'a> Board<'a> {
  pub fn new(board_id: &str, trello_api_key: &str, trello_oauth_token: &str, mongodb: &'a Database, slack_sender: &'a Sender) -> Board<'a> {
    Board {
      id: board_id.to_string(),
      name: "".to_string(),
      db: mongodb,
      sender: slack_sender,
      http_since_parameter: Utc::now(),
      http_url: format!("https://api.trello.com/1/boards/{}", board_id).to_string(),
      http_token_parameters: format!("key={}&token={}", trello_api_key, trello_oauth_token).to_string(),
      http_client: Client::new()
    }
  }

  pub fn listen(&mut self) -> Result<()> {
    let mut prop_resp = self.http_client
      .get(&format!("{}?fields=name&{}", self.http_url, self.http_token_parameters))
      .header(UserAgent::new(USER_AGENT.to_string()))
      .send()?;

    let properties : Value = prop_resp.json()?;
    self.name = properties.get("name").unwrap().as_str().unwrap().to_string();

    loop {
      let mut resp = self.http_client
        .get(&format!("{}/actions?filter={}&since={}&{}", self.http_url, ACTION_TYPE_FILTERS, self.http_since_parameter, self.http_token_parameters))
        .header(UserAgent::new(USER_AGENT.to_string()))
        .send()?;

      let actions : Vec<Action> = resp.json()?;

      for action in actions {
        if let Some(list_before) = action.data.get("listBefore") {
          let list_after = action.data.get("listAfter").unwrap();
          let list_before_name = list_before.get("name").unwrap().as_str().unwrap();
          let list_after_name = list_after.get("name").unwrap().as_str().unwrap();

          let trello_coll = self.db.collection("trello");
          let trello_lookup = doc! {
            "name": &action.creator.full_name
          };

          if let Some(tdoc) = trello_coll.find_one(Some(trello_lookup.clone()), None).expect("Failed to find document") {
            // TODO: Get list of trackers from tdoc and broadcast notification to each through Slack
          }
        }
      }

      self.http_since_parameter = Utc::now();

      thread::sleep(Duration::from_secs(600));
    }
  }
}