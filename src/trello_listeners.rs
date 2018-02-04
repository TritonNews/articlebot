use std::sync::mpsc::Sender;
use std::sync::{Mutex, Arc};
use std::error::Error;

use trello::CardHandler;
use trello_models::Action;
use mongodb::db::{Database, ThreadedDatabase};

pub trait ActionListener {
    fn get_filtered_actions(&self) -> &str;
    fn on_action(&self, action : &Action) -> Result<(), Box<Error>>;
}

pub struct RelayActionListener {
    db: Database,
    card_handler: CardHandler,
    buffer_tx: Sender<String>,
    buffer_count: Arc<Mutex<u8>>
}

impl RelayActionListener {
    pub fn new(db: Database, card_handler: CardHandler, buffer_tx: Sender<String>, buffer_count: Arc<Mutex<u8>>) -> RelayActionListener {
        RelayActionListener {
            db: db,
            card_handler: card_handler,
            buffer_tx: buffer_tx,
            buffer_count: buffer_count
        }
    }
}

impl ActionListener for RelayActionListener {
    fn get_filtered_actions(&self) -> &str {
        return &"updateCard";
    }

    fn on_action(&self, action : &Action) -> Result<(), Box<Error>> {
        // Make sure that we only capture when a card is moved between lists
        if let Some(list_before) = action.data.get("listBefore") {
            let card = self.card_handler.get_card(action.data.get("card").unwrap().get("id").unwrap().as_str().unwrap())?;
            let card_members = self.card_handler.get_card_members(&card)?;
            let card_title = card.name;

            let list_before_name = list_before.get("name").unwrap().as_str().unwrap();
            let list_after = action.data.get("listAfter").unwrap();
            let list_after_name = list_after.get("name").unwrap().as_str().unwrap();

            info!("Card \"{}\" was moved from \"{}\" to \"{}\".", card_title, list_before_name, list_after_name);

            for member in card_members {
                info!("Member \"{}\" is associated with this card.", &member.full_name);

                // If any Slack user is tracking this Trello user, find all Slack DM channel IDs through MongoDB and send a message to each one
                if let Some(tdoc) = self.db.collection("trello").find_one(Some(doc! {
                    "name": &member.full_name
                }), None)? {
                    let trackers = tdoc.get_array("trackers").unwrap();
                    for tracker in trackers {
                        // Tracker refers to a slack user that must exist
                        let sdoc = self.db.collection("slack").find_one(Some(doc! {
                            "uid": tracker.as_str().unwrap()
                        }), None)?.unwrap();
                        let channel = sdoc.get_str("cid").unwrap();

                        // Send the message by passing it to our mpsc sender
                        let message = format!("Your card \"{}\" has been moved from \"{}\" to \"{}\".", card_title, list_before_name, list_after_name);
                        self.buffer_tx.send(format!("{}|{}", channel, message))?;

                        // Increment the buffer count to notify the webhook that a flush needs to happen
                        let mut message_count = self.buffer_count.lock().unwrap();
                        *message_count += 1;
                    }
                }
            }
        }

        Ok(())
    }
}