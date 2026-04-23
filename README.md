# SecMon

A std-only high-performance security tool for monitoring various resources on multiple servers.

`hub` is the central server that receives data from `node` and sends commands to `node`.

`node` is a server that collects data and updates `hub`, and responds to commands from `hub`.

## Basic Usage

Start `hub` server with `secmon hub`.

Start `node` server with `HUB_IP=<ip> secmon node [who] [wg] [--reconnect]`. `[who]` `[wg]` selects which resources to monitor.

Various cli commands can be used on the `hub` server - see `secmon help` for more information.

## Notes

`hub` does not monitor its own resources, and so a separate `node` should be launched on the same server as `hub`.

Nodes check monitored resources every second, and update `hub` atomically once something changes.

All communication occurs in unencrypted tcp streams, as a trusted network is assumed.
