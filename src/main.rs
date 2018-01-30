extern crate slack_hook;
use slack_hook::{Slack, PayloadBuilder};

fn main() {
    let slack = Slack::new("https://hooks.slack.com/services/abc/123/45z").unwrap();
    let p = PayloadBuilder::new()
      .text("test message")
      .channel("#testing")
      .username("My Bot")
      .icon_emoji(":chart_with_upwards_trend:")
      .build()
      .unwrap();

    let res = slack.send(&p);
    match res {
        Ok(()) => println!("ok"),
        Err(x) => println!("ERR: {:?}",x)
    }
}