# SecMon: Core

The core `hub` and `node` daemon, plus a minimal viable implementation of cli client.

This package contains definitions of all models, traits and methods, which are public for integration use.

## Basic Usage

Modify and copy [`config.toml`](./src/config.toml) to `~/.config/secmon/config.toml` or `/etc/secmon/config.toml`, or use `-c /path/to/config.toml` at launch time.

Start `hub` daemon with `secmon hub`.

Start `node` daemon with `secmon node`.

The following commands can be used on the `hub` server:

- `secmon subscribe`: subscribe to node updates - mostly for debug purpose, or if you want to watch terminal print things
- `secmon <list|->`: print info of all nodes
- `secmon <node>`: print info of one node
- `secmon <node|-> <command-label>`: execute a command on one/all node(s) and stream output
- `secmon <group>`: print info of all nodes in group
- `secmon <group> <command-label>`: execute a command on all nodes in group and stream output

`<group>` is set in node's config, since all nodes are assumed trusted either by secure network or by noise protocol. If a query can identify both a group and a node (e.g. a group's name is also a node's hostname), then the group takes precedence (i.e. individual matched nodes are omitted, unless the node is also in the matched group).

Note that no arbitrary command execution is allowed and each node must explicitly configure an allowlist for commands. Also, no command arguments are allowed to prevent non-whitelisted commands from being maliciously injected by abusing arguments. 

See `secmon help` for detailed information on using the program.

## Crypto

Noise protocol with `Noise_XKpsk0_25519_ChaChaPoly_BLAKE2s` pattern is supported for connections over insecure networks.

To filter unwanted/malicious nodes, either preshared key or node whitelist (or both) shall be configured. Preshared key encrypts all communications from the first message, and so unauthorized nodes cannot complete a handshake at all. On the other hand, node whitelist checks node's public key after handshake, and drops connection if the key is not whitelisted.

A helper command `secmon genkey <pair|psk>` is provided to conveniently generate key pairs and preshared keys.

Details about the implementation can be found in [`crypto.rs`](./src/crypto.rs).

## Notes

`hub` does not monitor its own resources, and so a separate `node` should be launched on the same server as `hub`.

Nodes check `who` and `wg` every second, and watch for changes on `auth` with `journalctl -f`, and update `hub` atomically once something changes.

Ubuntu 26.04 dropped support for `who` where it outputs nothing and the current solution is to use `gnuwho`. Node would try to use `gnuwho` first and fall back to `who` if `gnuwho` does not exist (because node cannot tell if empty output from `who` means there is no active sessions, or if `who` is broken).
