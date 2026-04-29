# SecMon: Core

The core `hub` and `node` daemon, plus a minimal viable implementation of cli client.

This module contains definitions of all models, traits and methods, which are public for integration use.

## Basic Usage

Start `hub` daemon with `secmon hub`.

Start `node` daemon with `secmon node [who] [wg] [auth] [--reconnect]`.

- `[who]` `[wg]` `[auth]` selects the resources to monitor.

The following commands can be used on the `hub` server:

- `secmon list [sorted]`: list all connected (and recently disconnected) nodes
- `secmon subscribe`: subscribe to node updates - mostly for debug purpose, or if you want to watch terminal print things
- `secmon <node> execute <label>`: execute an allowed command on one/several node(s) and stream output

See `secmon help` for detailed information on using the program.

## Notes

`hub` does not monitor its own resources, and so a separate `node` should be launched on the same server as `hub`.

Nodes check `who` and `wg` every second, and watch for changes on `auth` with `journalctl -f`, and update `hub` atomically once something changes.

All communication occurs in unencrypted tcp streams, as a trusted network is assumed. A secure tunnel should be used for communication over the Internet.
