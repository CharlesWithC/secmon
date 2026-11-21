# SecMon: Telegram

Telegram bot integration for single-user control and updates.

This is a minimal viable implementation that sends ssh/su updates, allows user to list nodes and remotely execute commands. Advanced features such as streaming result of remote command execution are not supported, as the focus of the project is not to develop a perfect Telegram bot.

## Basic Usage

Configure required `token` and `user_id` in `config.toml`. Optionally, modify enabled features with `features` attribute in config.

- `user_id` refers to the user that is authorized to use the bot. This user would receive all node updates and be able to execute commands. Only a single user is allowed.

See `secmon-tg help` for detailed information on using the program.

## Notes

For obvious reasons, `secmon-tg` must run on the same server and under the same user as `secmon hub`, as they communicate over unix socket.
