# SecMon

A std-only high-performance security tool for monitoring various resources on multiple servers.

`hub` is the central server that receives data from `node` and sends commands to `node`.

`node` is a server that collects data and updates `hub`, and responds to commands from `hub`.

## Basic Usage

Start `hub` server with `secmon hub`.

Start `node` server with `secmon node [who] [wg] [auth] [--reconnect]`. `[who]` `[wg]` `[auth]` selects the resources to monitor.

Various helpful client commands can be used on the `hub` server with cli.

See `secmon help` for detailed information on using the program.

## Data Monitored

Two types of data are monitored: `stored` and `tracked-but-not-stored` (aka `tracked-only`). Stored data is part of the persistent node state and may be fetched at any time. Tracked-only data is only broadcasted to subscribed clients, and is not stored by `hub` or `node` (that said, client might store such data if needed).

This design aligns with the principle of this project of being a "monitor" rather than a "log viewer". That is, the project is supposed to monitor and notify changes, rather than serve as a portal to read remote logs.

`who` and `wg` data are stored because they are polled and thus must be stored to compare changes. The "storage" is only a by-product of polling, while the data just turns out to be interesting to be fetched and viewed in cli.

`auth` (log) data is inherently "ephemeral" and it is also hard to track the exact state of current logins by reading log (technically, it is possible, but building such state is too fragile compared to the stable data printed by `who` or `wg`). Also, it is much more interesting to subscribe to such data for updates and ping the end-user when a login occurs, than to, say, "view last 10 successful logins".

## Notes

`hub` does not monitor its own resources, and so a separate `node` should be launched on the same server as `hub`.

Nodes check `who` and `wg` every second, and watch for changes on `auth`, and update `hub` atomically once something changes.

All communication occurs in unencrypted tcp streams, as a trusted network is assumed.
