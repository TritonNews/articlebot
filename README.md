# articlebot

<p align="center">
  <img src="https://github.com/TritonNews/articlebot/raw/master/icon.png"/>
</p>

This bot notifies editors on Slack when their corresponding Trello articles/cards have been moved between lists. It is written entirely in Rust (the best programming language ever). The bot currently lives as a background task on a DigitalOcean Droplet instance, but we will be looking to adjust our hosting solution in the future.

## Interacting with _articlebot_

You can interact with articlebot through commands. Commands are given in the form "{COMMAND} {ARGUMENTS}" where {COMMAND} is some word and {ARGUMENTS} is some space-delimited list of words.

Here is a list of valid commands:

* hello
  - Responds with a greeting.

* whoami
  - Prints out your information, as recorded in articlebot's database.

* track {TRELLO USER}
  - Tells articlebot that you wish to follow card movements for {TRELLO USER} on Trello.
  - e.g. "/track John Doe" would enable notifications for John Doe's Trello cards.

## Build Process

Coming soon!

## How _articlebot_ Works

Coming soon!

<sup><sub>Icon made by <a href="https://www.flaticon.com/authors/smashicons" title="Smashicons">Smashicons</a> at <a href="https://www.flaticon.com/" title="Flaticon">www.flaticon.com</a> and licensed under <a href="http://creativecommons.org/licenses/by/3.0/" title="Creative Commons BY 3.0" target="_blank">CC 3.0 BY</a></sub></sup>
