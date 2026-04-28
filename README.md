# SecMon

An std-only high-performance tool for monitoring security data on multiple servers.

This is a mini distributed system experiment, where remote nodes communicates with hub, and hub communicates with terminal clients.

```text
Client 1 <---UDS---> Hub <---TCP---> Node 1
                     ^ ^
Client 2 <-----------| |-----------> Node 2
```

`hub` is the central server, it:

- receives updates from remote nodes via tcp socket
- broadcasts updates from all nodes to subscribed terminal clients via unix socket
- accepts commands from terminal clients, forwards commands to node, and then forwards responses back to client

`node` is the server being monitored, it:

- collects data and updates hub atomically on changed data
- responds to commands from hub, such as executing an allowed shell command

`client` is the terminal client, it:

- communicates with hub via unix socket, and so must run on the same server as hub
- allows integration to be built based on the custom protocol; the cli client is a minimal viable implementation

## Basic Usage

Start `hub` server with `secmon hub`.

Start `node` server with `secmon node [who] [wg] [auth] [--reconnect]`.

- `[who]` `[wg]` `[auth]` selects the resources to monitor.

The following commands can be used on the `hub` server:

- `secmon list [sorted]`: list all connected (and recently disconnected) nodes
- `secmon subscribe`: subscribe to node updates - mostly for debug purpose, or if you want to watch terminal print things
- `secmon <node> execute <label>`: execute an allowed command on one/several node(s)

See `secmon help` for detailed information on using the program.

## The Philosophy (Data Monitored)

Two types of data are monitored: `stored` and `tracked-but-not-stored` (aka `tracked-only`). Stored data is part of the persistent node state and may be fetched at any time, and is also broadcasted to subscribed clients on updates. Tracked-only data is only broadcasted to subscribed clients, and is not stored by `hub` or `node` (that said, client might store such data if needed).

This design aligns with the principle of this project of being a "monitor" rather than a "log viewer". That is, the project is supposed to monitor and notify changes, rather than serve as a portal to read remote logs.

`who` and `wg` data are stored because they are polled and thus must be stored to compare changes. The "storage" is a by-product of polling, despite the data turning out to be interesting to be stored for efficient fetching/displaying in cli client.

`auth` (log) data is inherently "ephemeral" - it is hard to track the exact state of current sessions by reading log (technically, it is possible, but building such state is too fragile compared with using the stable data printed by `who` or `wg`). Also, it is much more interesting to subscribe to such data for updates and ping the terminal client when a login occurs, than to, say, "view last 10 successful logins".

## Notes

`hub` does not monitor its own resources, and so a separate `node` should be launched on the same server as `hub`.

Nodes check `who` and `wg` every second, and watch for changes on `auth` with `journalctl -f`, and update `hub` atomically once something changes.

All communication occurs in unencrypted tcp streams, as a trusted network is assumed.
