use std::time::Duration;
use std::thread;
use reqwest::Client;
use reqwest::header::UserAgent;
use reqwest::Result;
use serde_json::Value;
use chrono::prelude::*;

const USER_AGENT: &'static str = "Mozilla/5.0 (Windows NT 5.1; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/46.0.2486.0 Safari/537.36 Edge/13.10586";
const UPDATE_INTERVAL_SECONDS: u64 = 600;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    pub id: String,
    pub avatar_hash: String,
    pub full_name: String,
    pub initials: String,
    pub username: String
}

#[derive(Deserialize)]
pub struct Action {
    pub id: String,
    pub data: Value,
    pub date: String, // TODO: Switch to using DateTime<Utc> later
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(rename = "idMemberCreator")]
    pub creator_id: String,
    #[serde(rename = "memberCreator")]
    pub creator: Member
}

pub trait BoardListener {
    fn on_action(&self, action : Action);
}

pub struct BoardHandler<L> {
    pub id: String,
    board_listener: L,
    http_url: String,
    http_since_parameter: DateTime<Utc>,
    http_token_parameters: String,
    http_client: Client
}

impl<L : BoardListener> BoardHandler<L> {
    pub fn new(board_id: &str, trello_api_key: &str, trello_oauth_token: &str, board_listener : L) -> BoardHandler<L> {
        BoardHandler {
            id: board_id.to_string(),
            board_listener: board_listener,
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
                self.board_listener.on_action(action);
            }

            self.http_since_parameter = Utc::now();

            thread::sleep(Duration::from_secs(UPDATE_INTERVAL_SECONDS));
        }
    }
}