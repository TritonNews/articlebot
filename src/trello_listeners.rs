
use trello::{BoardListener, get_card, get_card_members};
use trello_models::Action;
use slack::Sender;
use mongodb::db::{Database, ThreadedDatabase};
use reqwest::Client;

pub struct RelayBoardListener {
    db: Database,
    sender: Sender,
    http_token_parameters: String,
    http_client: Client
}

impl RelayBoardListener {
    pub fn new(db : Database, slack_sender : Sender, trello_api_key : &str, trello_oauth_token : &str) -> RelayBoardListener {
        RelayBoardListener {
            db: db,
            sender: slack_sender,
            http_token_parameters: format!("key={}&token={}", trello_api_key, trello_oauth_token).to_string(),
            http_client: Client::new()
        }
    }
}

impl BoardListener for RelayBoardListener {
    fn get_filtered_actions(&self) -> &str {
        return &"updateCard";
    }

    fn on_action(&self, action : &Action) {
        // Make sure that we only capture when a card is moved between lists
        if let Some(list_before) = action.data.get("listBefore") {
            let card = get_card(action.data.get("card").unwrap().get("id").unwrap().as_str().unwrap(),
                &self.http_token_parameters[..], &self.http_client).expect("Trello card error");
            let card_members = get_card_members(&card, &self.http_token_parameters[..], &self.http_client).expect("Trello card error");
            let card_title = card.name;

            let list_before_name = list_before.get("name").unwrap().as_str().unwrap();
            let list_after = action.data.get("listAfter").unwrap();
            let list_after_name = list_after.get("name").unwrap().as_str().unwrap();

            info!("Card \"{}\" was moved from \"{}\" to \"{}\".", card_title, list_before_name, list_after_name);

            for member in card_members {
                let trello_coll = self.db.collection("trello");
                let trello_lookup = doc! {
                    "name": &member.full_name
                };

                info!("Member \"{}\" is associated with this card.", &member.full_name);

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

                      self.sender.send_typing(channel);
                      self.sender.send_message(channel, &format!("Your card \"{}\" has been moved from \"{}\" to \"{}\".", card_title, list_before_name, list_after_name))
                        .expect("Slack sender error");
                    }
                }
            }
        }
    }
}