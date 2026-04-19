# SecMon

A security tool for monitoring various resources on multiple servers.

`hub` is the central server that receives data from `node` and sends commands to `node`.

`node` is a server that collects data and updates `hub`, and responds to commands from `hub`.

## Basic Usage

Start `hub` server with `secmon hub`.

Start `node` server with `HUB_IP=<ip> secmon node [who] [wg] [--reconnect]`.

Note that `hub` does not monitor its own resources, and so a `node` may be launched on the same server as `hub`.

The resources to be monitored is selected with positional arguments `[who]`, `[wg]`. A resource will not be monitored if the corresponding argument is not provided.

Nodes will check for monitored resources every second, and update `hub` once something is updated.

[TODO] Node should only update hub on what exactly is changed - i.e. node should not sync back states on all resources if only one changes.

[TODO] Various commands may be used on the `hub` server to view status and manage `node`.

## Notes

All communication occurs in unencrypted tcp streams, as a trusted network is assumed.
