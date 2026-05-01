# SecMon: HTTP

HTTP server integration that exposes unix-socket-based hub command interface over HTTP.

This integration should allow clients written in other programming languages to easily communicate with the hub daemon, as the HTTP server handles translation between json serialization and postcard binary serialization.

This package also exports client util functions for telegram and discord integrations.

## Basic Usage

Start http server with `secmon-http`.

The server binds to `127.0.0.1:9993` by default, but you may change it with `SERVER_IP` and `SERVER_PORT` environment variables.

Sample requests:

```bash
# subscribe to node updates (websocket)
curl --no-buffer --include \
     --header "Connection: Upgrade" \
     --header "Upgrade: websocket" \
     --header "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
     --header "Sec-WebSocket-Version: 13" \
     --output - \
     http://localhost:9993/subscribe

# lists all nodes
curl http://localhost:9993/list

# fetches info about 'fury'
curl http://localhost:9993/fury

# fetches node state from 'fury'
curl -X POST \
     -H "Content-Type: application/json" \
     -d '"NodeState"' \
     http://localhost:9993/fury/execute

# executes a raw command on fury
# note: streaming response not supported
curl -X POST \
     -H "Content-Type: application/json" \
     -d '{"Execute": { "command_label": "reboot", "stream": false }}' \
     http://localhost:9993/fury/execute
```

See `secmon-http help` for detailed information on using the program.

## Notes

For obvious reasons, `secmon-http` must run on the same server and under the same user as `secmon hub`, as they communicate over unix socket.

The HTTP server does not support authentication, and so it should only be exposed on a trusted network.
