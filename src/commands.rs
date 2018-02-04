use std::error::Error;
use std::env;

use slack::RtmClient;
use slack_api::MessageStandard;
use mongodb::db::{Database, ThreadedDatabase};
use bson::Bson;

pub struct CommandHandler {
    db: Database
}

impl CommandHandler {
    pub fn new(db: Database) -> CommandHandler {
        CommandHandler {
            db: db
        }
    }

    pub fn handle_message(&self, message: MessageStandard, cli: &RtmClient) -> Result<(), Box<Error>> {
        let text : &str = &message.text.unwrap()[..];
        let channel : &str = &message.channel.unwrap()[..];
        let user : &str = &message.user.unwrap()[..];
        let split_text: Vec<&str> = text.split(" ").collect();
        let command = split_text[0].to_lowercase();
        let args = &split_text[1..];

        info!("Message from {}: {}", user, text);
        info!("Interpreting as COMMAND={} ARGUMENTS={:?}", command, args);

        self.on_command(&command[..], args, user, channel, cli)
    }

    fn on_command(&self, command: &str, args: &[&str], user: &str, channel: &str, cli: &RtmClient) -> Result<(), Box<Error>> {
        let sender = cli.sender();

        if command == "hello" || command == "hi" {
            sender.send_message(channel, "Hello there.")?;
        }
        else if command == "version" || command == "v" {
            sender.send_message(channel, &format!("Running v{}.", env::var("CARGO_PKG_VERSION")?)[..])?;
        }
        else if command == "help" {

        }
        else if command == "tracking" {
            sender.send_message(channel, "Fetching user information ...")?;
            if let Some(sdoc) = self.db.collection("slack").find_one(Some(doc! {
                "uid": user
            }), None)? {
                sender.send_message(channel, &format!("You are currently tracking \"{}\" on Trello.", sdoc.get_str("tracking").unwrap())[..])?;
            }
            else {
                sender.send_message(channel, "You are currently not tracking a Trello user.")?;
            }
        }
        else if command == "track" {
            let tracker = user;
            let tracking = args.join(" ");

            // Slack collection in MongoDB (key: tracker id, other data: channel id, tracking name)
            let slack_coll = self.db.collection("slack");
            let slack_lookup = doc! {
                "uid": tracker
            };

            // Trello collection in MongoDB (key: tracking name, other data: list of trackers)
            let trello_coll = self.db.collection("trello");
            let trello_lookup = doc! {
                "name": &tracking
            };

            // Delete all previous records in the Slack and Trello collections if they exist
            if let Some(sdoc) = slack_coll.find_one_and_delete(slack_lookup, None)? {
                let trello_lookup_old = doc! {
                    "name": sdoc.get_str("tracking").unwrap()
                };
                if let Some(tdoc) = trello_coll.find_one(Some(trello_lookup_old.clone()), None)? {
                    // Remove our current tracker from where it was originally tracking
                    let mut trackers_old = tdoc.get_array("trackers").unwrap().clone();
                    let index = trackers_old.iter().position(|tracker_old| *tracker_old.as_str().unwrap() == tracker.to_string()).unwrap();
                    trackers_old.remove(index);

                    // If the Trello user has no trackers, delete it. Otherwise, update it to reflect the changes made to its changes.
                    if trackers_old.is_empty() {
                        trello_coll.delete_one(trello_lookup_old, None)?;
                    }
                    else {
                        let mut tdoc_new = tdoc.clone();
                        tdoc_new.insert_bson("trackers".to_string(), Bson::Array(trackers_old));
                        trello_coll.update_one(trello_lookup_old, tdoc_new, None)?;
                    }
                }
            }

            // Insert a new Slack document specifying the tracker's information
            slack_coll.insert_one(doc! {
                "uid": tracker,
                "cid": channel,
                "tracking": &tracking
            }, None)?;

            // Update (or create/insert) the Trello document that contains the trackers
            if let Some(tdoc) = trello_coll.find_one(Some(trello_lookup.clone()), None)? {
                let mut trackers = tdoc.get_array("trackers").unwrap().clone();
                trackers.push(Bson::String(tracker.to_string()));

                let mut tdoc_new = tdoc.clone();
                tdoc_new.insert_bson("trackers".to_string(), Bson::Array(trackers));

                trello_coll.update_one(trello_lookup, tdoc_new, None)?;
            }
            else {
                trello_coll.insert_one(doc! {
                    "name": &tracking,
                    "trackers": [tracker]
                }, None)?;
            }

            sender.send_message(channel, &format!("You will now be notified when {}'s articles are moved in Trello.", tracking)[..])?;
        }
        else {
            sender.send_message(channel, &format!("I did not understand your command \"{}\".", command)[..])?;
        }

        Ok(())
    }
}

