# SecMon: Telegram

Telegram bot integration for single-user control and updates.

## Basic Usage

Configure mandatory environment variables: `TELEGRAM_BOT_TOKEN` and `TELEGRAM_USER_ID`.

- `TELEGRAM_USER_ID` refers to the user that is authorized to use the bot. This user would receive all node updates and be able to execute commands. Only a single user is allowed.

The following environment variables may be configured optionally:

- `IPV4_ONLY=<true|false>`
- `COMMAND_ALLOWLIST_FILE=<path>`

Then, start the bot with `secmon-tg [upd] [exec]`.

- `[upd]` decides whether to send the authorized user node updates on SSH and SU sessions.
- `[exec]` decides whether the authorized user is allowed to execute commands remotely.

See `secmon-tg help` for detailed information on using the program.
