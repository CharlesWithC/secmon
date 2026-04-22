# SecMon

A security tool for monitoring various resources on multiple servers.

`hub` is the central server that receives data from `node` and sends commands to `node`.

`node` is a server that collects data and updates `hub`, and responds to commands from `hub`.

## Basic Usage

Start `hub` server with `secmon hub`.

Start `node` server with `HUB_IP=<ip> secmon node [who] [wg] [--reconnect]`.

Various management commands can be used on the server running `hub` - see `secmon help` for more information.

Note that `hub` does not monitor its own resources, and so a `node` may be launched on the same server as `hub`.

The resources to be monitored is selected with positional arguments `[who]`, `[wg]`. Resource is monitored if the corresponding argument is not provided.

Nodes will check monitored resources every second, and update `hub` once something changes.

## Notes

All communication occurs in unencrypted tcp streams, as a trusted network is assumed.
