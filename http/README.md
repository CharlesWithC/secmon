# SecMon: HTTP

HTTP server integration that exposes unix-socket-based hub command interface over HTTP.

This integration should allow clients written in other programming languages to easily communicate with the hub daemon, as the HTTP server handles translating json serialization to postcard binary serialization.

This integration is currently under development.

## Basic Usage

Start http server with `secmon-http`.

The server binds to `127.0.0.1:9993` by default, but you may change it with `SERVER_IP` and `SERVER_PORT` environment variables.

See `secmon-http help` for detailed information on using the program.

## Notes

For obvious reasons, `secmon-http` must run on the same server and under the same user as `secmon hub`, as they communicate over unix socket.
