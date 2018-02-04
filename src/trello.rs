use std::time::Duration;
use std::{env, thread};
use reqwest::Client;
use reqwest::header::UserAgent;
use reqwest::Result;
use chrono::prelude::*;
use trello_models::*;

const API_URL: &'static str = "https://api.trello.com/1";
const USER_AGENT: &'static str = "Mozilla/5.0 (Windows NT 5.1; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/46.0.2486.0 Safari/537.36 Edge/13.10586";
const UPDATE_INTERVAL_SECONDS: u64 = 60;

pub trait BoardListener {
    fn get_filtered_actions(&self) -> &str;
    fn on_action(&self, action : &Action);
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
            http_url: format!("{}/boards/{}", API_URL, board_id).to_string(),
            http_token_parameters: format!("key={}&token={}", trello_api_key, trello_oauth_token).to_string(),
            http_client: Client::new()
        }
    }

    pub fn listen(&mut self) -> Result<()> {
        info!("v{} listening for updates.", env::var("CARGO_PKG_VERSION").unwrap());
        loop {
            let url = format!("{}/actions?filter={}&since={}&{}", self.http_url, self.board_listener.get_filtered_actions(), self.http_since_parameter, self.http_token_parameters);

            info!("Pinging board ... {}", url);

            let mut resp = self.http_client
                .get(&url)
                .header(UserAgent::new(USER_AGENT.to_string()))
                .send()?;

            let actions : Vec<Action> = resp.json()?;

            info!("{} actions since last update.", actions.iter().count());

            for action in actions.iter().rev() {
                self.board_listener.on_action(action);
            }

            self.http_since_parameter = Utc::now();

            info!("Thread sleeping for {} seconds ...", UPDATE_INTERVAL_SECONDS);

            thread::sleep(Duration::from_secs(UPDATE_INTERVAL_SECONDS));
        }
    }
}

// TODO: Remove this gimmicky solution and replace with a CardHandler
pub fn get_card_members(card_id: &str, http_token_parameters: &str, http_client: &Client) -> Result<Vec<Member>> {
    let card_url = format!("{}/cards/{}?fields=all&{}", API_URL, card_id, http_token_parameters);

    info!("Fetching card ... {}", card_url);

    let mut card_resp = http_client
        .get(&card_url)
        .header(UserAgent::new(USER_AGENT.to_string()))
        .send()?;

    let card : Card = card_resp.json()?;

    let mut members = Vec::new();
    for member_id in card.id_members {
        let member_url = format!("{}/members/{}?fields=all&{}", API_URL, member_id, http_token_parameters);

        let mut member_resp = http_client
            .get(&member_url)
            .header(UserAgent::new(USER_AGENT.to_string()))
            .send()?;

        let member : Member = member_resp.json()?;

        members.push(member);
    }

    Ok(members)
}