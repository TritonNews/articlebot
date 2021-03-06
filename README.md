# articlebot

<p align="center">
  <img src="https://github.com/TritonNews/articlebot/raw/master/icon.png"/>
</p>

This bot notifies editors on Slack when their corresponding Trello articles/cards have been moved between lists. It is written entirely in Rust (the best programming language ever). The bot currently lives as a background task on a DigitalOcean Droplet instance, but we will be looking to adjust our hosting solution in the future.

## Usage

You can interact with articlebot through commands. Commands are given in the form [COMMAND] [ARGUMENTS] where [COMMAND] is the first word in your query and [ARGUMENTS] is whatever comes after it.

An example of a valid command would be `track johndoe42`. In this case, the command is `track` and the arguments are `johndoe42`.

Here is a list of valid commands:

* `hello` or `hi` displays a greeting.
* `version` displays articlebot's current version.
* `tutorial` displays an overview of articlebot's command system.
* `help` displays a list of available commands.
* `tracking` displays who you are following on Trello, as recorded in articlebot's database.
* `track [USERNAME]` tells articlebot that you wish to follow card movements for [USERNAME] on Trello.
  - [USERNAME] must be exact or articlebot will not return any notifications to you.
* `untrack` tells articlebot that you no longer wish to unfollow any Trello user you might have been following

## Build Process

Prerequisites:

* make
* Rust
* MongoDB (instance must be running @ localhost:27017)
* A Slack site (apps & webhooks must be enabled and the bot must be in the same channel as the webhook)
* A Trello board (user must have read permissions)

These environment variables will need to have been appropriately filled in:

* SLACK_API_KEY
* SLACK_WEBHOOK
* TRELLO_API_KEY
* TRELLO_OAUTH_TOKEN
* TRELLO_BOARD_ID

After verifying that the above prerequisites have been satisfied, you can begin deploying articlebot. Simply run `make release` and the relevant packages will be built. Once the build process has completed, articlebot will run as a background task and pipe its output to the most recent log file under logs/. If you wish to run articlebot attached to your shell, you can use `RUST_LOG=info cargo run` or `make test` depending on the level of log output you desire.

<sup><sub>Icon made by <a href="https://www.flaticon.com/authors/smashicons" title="Smashicons">Smashicons</a> at <a href="https://www.flaticon.com/" title="Flaticon">www.flaticon.com</a> and licensed under <a href="http://creativecommons.org/licenses/by/3.0/" title="Creative Commons BY 3.0" target="_blank">CC 3.0 BY</a></sub></sup>
