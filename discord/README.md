# SecMon: Discord

Discord webhook integration for node updates.

This is a minimal viable implementation that sends ssh/su updates to a single webhook. Advanced features such as supporting multiple webhooks are not supported, as the focus of the project is not to develop a perfect Discord integration.

## Basic Usage

Configure mandatory environment variable: `DISCORD_WEBHOOK_URL`.

The following environment variable may be configured optionally:

- `DISCORD_MESSAGE_CONTENT=<content>`

Then, start the integration with `secmon-dc`.
