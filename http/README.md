# SecMon: HTTP

HTTP server integration that exposes unix-socket-based hub command interface over HTTP.

This integration should allow clients written in other programming languages to easily communicate with the hub daemon, as the HTTP server handles translation between json serialization and postcard binary serialization.

## Basic Usage

Start http server with `secmon-http`.

The server binds to `127.0.0.1:9993` by default, but you may change it with `SERVER_IP` and `SERVER_PORT` environment variables.

Sample requests:

```bash
curl http://localhost:9993/subscribe            # subscribe to node updates (websocket)
curl http://localhost:9993/list                 # lists all nodes
curl http://localhost:9993/fury                 # fetchs info about 'fury'
curl -X POST \
     -H "Content-Type: application/json" \
     -d '"NodeState"' \                         # fetches node state from 'fury'
     http://localhost:9993/fury/execute         # note: demo purpose; not recommended
curl -X POST \
     -H "Content-Type: application/json" \
     -d '{"Execute": ["reboot", false]}' \      # executes a raw command on fury
     http://localhost:9993/fury/execute         # note: streamed response not supported
```

See `secmon-http help` for detailed information on using the program.

## Notes

For obvious reasons, `secmon-http` must run on the same server and under the same user as `secmon hub`, as they communicate over unix socket.

The HTTP server does not support authentication, and so it should only be exposed on a trusted network.
