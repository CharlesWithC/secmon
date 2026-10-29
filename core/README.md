# SecMon: Core

The core `hub` and `node` daemon, plus a minimal viable implementation of cli client.

This package contains definitions of all models, traits and methods, which are public for integration use.

## Basic Usage

Modify and copy [`config.toml`](./src/config.toml) to `~/.config/secmon/config.toml` or `/etc/secmon/config.toml`, or use `-c /path/to/config.toml` at launch time.

Start `hub` daemon with `secmon hub`.

Start `node` daemon with `secmon node`.

The following commands can be used on the `hub` server:

- `secmon subscribe`: subscribe to node updates - mostly for debug purpose, or if you want to watch terminal print things
- `secmon <list|->`: list all recently connected nodes
- `secmon <node>`: show info about a recently connected node
- `secmon <node> <command-label>`: execute an allowed command on one/several node(s) and stream output

See `secmon help` for detailed information on using the program.

## Notes

`hub` does not monitor its own resources, and so a separate `node` should be launched on the same server as `hub`.

Nodes check `who` and `wg` every second, and watch for changes on `auth` with `journalctl -f`, and update `hub` atomically once something changes.

All communication occurs in unencrypted tcp streams, as a trusted network is assumed. A secure tunnel should be used for communication over the Internet.
