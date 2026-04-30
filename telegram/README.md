# SecMon: Telegram

Telegram bot integration for single-user control and updates.

This is a minimal viable implementation that sends ssh/su updates, allows user to list nodes and remotely execute commands. Advanced features such as streaming result of remote command execution is not supported, as the focus of the project is not to develop a perfect Telegram bot.

## Basic Usage

Configure mandatory environment variables: `TELEGRAM_BOT_TOKEN` and `TELEGRAM_USER_ID`.

- `TELEGRAM_USER_ID` refers to the user that is authorized to use the bot. This user would receive all node updates and be able to execute commands. Only a single user is allowed.

The following environment variables may be configured optionally:

- `IPV4ONLY=<true|false>`
- `TIMEZONE=<timezone>`
- `COMMAND_ALLOWLIST_FILE=<path>`

Then, start the bot with `secmon-tg [upd] [exec]`.

- `[upd]` decides whether to send the authorized user node updates on SSH and SU sessions.
- `[exec]` decides whether the authorized user is allowed to execute commands remotely.

See `secmon-tg help` for detailed information on using the program.

## Notes

For obvious reasons, `secmon-tg` must run on the same server and under the same user as `secmon hub`, as they communicate over unix socket.
