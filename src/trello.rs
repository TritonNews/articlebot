use reqwest::Client;
use reqwest::header::UserAgent;
use reqwest::Result;
use slack::Sender;

const USER_AGENT: &'static str =
  "Mozilla/5.0 (Windows NT 5.1; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/46.0.2486.0 Safari/537.36 Edge/13.10586";

#[derive(Deserialize)]
struct BoardProperties {
  id: String,
  name: String
}

pub struct Board<'a> {
  properties: BoardProperties,
  slack_sender: &'a Sender,
  http_url: String,
  http_token_parameters: String,
  http_client: Client
}

impl<'a> Board<'a> {

  pub fn new(board_id: &str, trello_api_key: &str, trello_oauth_token: &str, slack_sender: &'a Sender) -> Board<'a> {
    let properties = BoardProperties {
      id: board_id.to_string(),
      name: "".to_string()
    };

    Board {
      properties: properties,
      slack_sender: slack_sender,
      http_url: format!("https://api.trello.com/1/boards/{}", board_id).to_string(),
      http_token_parameters: format!("key={}&token={}", trello_api_key, trello_oauth_token).to_string(),
      http_client: Client::new()
    }
  }

  pub fn listen(&mut self) -> Result<()> {
    let mut resp = self.http_client
      .get(&format!("{}?fields=name&{}", self.http_url, self.http_token_parameters))
      .header(UserAgent::new(USER_AGENT.to_string()))
      .send()?;

    self.properties = resp.json()?;

    Ok(())
  }
}