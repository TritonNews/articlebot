# articlebot

<p align="center">
  <img src="https://github.com/TritonNews/articlebot/raw/master/icon.png"/>
</p>

This bot notifies editors on Slack when their corresponding Trello articles/cards have been moved between lists. It is written entirely in Rust (the best programming language ever). The bot currently lives as a background task on a DigitalOcean Droplet instance, but we will be looking to adjust our hosting solution in the future.

## Interacting with _articlebot_

You can interact with articlebot through commands. Commands are given in the form [COMMAND] [ARGUMENTS] where [COMMAND] is the first word in your query and [ARGUMENTS] is whatever comes after it.

An example of a valid command would be `track johndoe42`. In this case, the command is `track` and the arguments are **johndoe42**.

Here is a list of valid commands:

* `hello` or `hi` displays a greeting.
* `version` displays articlebot's current version.
* `tutorial` displays an overview of articlebot's command system.
* `help` displays a list of available commands.
* `tracking` displays who you are following on Trello, as recorded in articlebot's database.
* `track [USERNAME]` tells articlebot that you wish to follow card movements for [USERNAME] on Trello.
  - [USERNAME] must be exact or articlebot will not return any notifications to you.

## Build Process

Coming soon!

## How _articlebot_ Works

Coming soon!

<sup><sub>Icon made by <a href="https://www.flaticon.com/authors/smashicons" title="Smashicons">Smashicons</a> at <a href="https://www.flaticon.com/" title="Flaticon">www.flaticon.com</a> and licensed under <a href="http://creativecommons.org/licenses/by/3.0/" title="Creative Commons BY 3.0" target="_blank">CC 3.0 BY</a></sub></sup>
