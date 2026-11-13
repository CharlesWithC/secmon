# SecMon: Discord

Discord webhook integration for node updates.

This is a minimal viable implementation that sends ssh/su updates to a single webhook. Advanced features such as multiple webhooks are not supported, as the focus of the project is not to develop a perfect Discord integration.

## Basic Usage

Configure required `webhook` and optional `message` in `config.toml`.

Then, start the integration with `secmon-dc -c config.toml`.

## Notes

For obvious reasons, `secmon-dc` must run on the same server and under the same user as `secmon hub`, as they communicate over unix socket.
