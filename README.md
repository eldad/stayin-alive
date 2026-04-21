# stayin-alive
Keep alive endpoint for load testing concurrent sessions

## Endpoints

The server exposes two ports:

| Protocol | Port |
|----------|------|
| HTTP     | 3000 |
| gRPC     | 50051 |

---

### HTTP

#### `GET /ping`

Simple ping/pong — returns `pong` immediately.

```bash
curl http://localhost:3000/ping
```

---

#### `GET /sse-ping`

Server-Sent Events stream. The server emits a `ping` event at the requested
interval and keeps the connection open indefinitely.

| Query parameter | Default | Description                              |
|-----------------|---------|------------------------------------------|
| `interval_ms`   | `1000`  | Milliseconds between consecutive events |

```bash
# Stream events at the default interval (1 s)
curl -N "http://localhost:3000/sse-ping"

# Stream events every 500 ms
curl -N "http://localhost:3000/sse-ping?interval_ms=500"
```

---

#### `GET /ws`

WebSocket endpoint. The server sends a `ping` text message at the requested
interval and keeps the connection open indefinitely.

| Query parameter | Default | Description                                |
|-----------------|---------|---------------------------------------------|
| `interval_ms`   | `1000`  | Milliseconds between consecutive ping frames |

```bash
# Initiate a WebSocket upgrade (raw frames will follow in the terminal)
curl -N \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
  -H "Sec-WebSocket-Version: 13" \
  "http://localhost:3000/ws?interval_ms=1000"
```

---

### gRPC

The gRPC server listens on port `50051`.
Examples below use [grpcurl](https://github.com/fullstorydev/grpcurl) and
assume the repository root is the working directory so the proto file can be
located.

#### `stayin_alive.StayinAlive/Ping`

Simple ping/pong — returns immediately.

```bash
grpcurl -plaintext \
  -import-path ./proto -proto stayin_alive.proto \
  localhost:50051 stayin_alive.StayinAlive/Ping
```

---

#### `stayin_alive.StayinAlive/PingMeLater`

Delayed ping — the server waits `delay_ms` milliseconds before replying.

| Field      | Type     | Description                              |
|------------|----------|------------------------------------------|
| `delay_ms` | `uint64` | Delay in milliseconds before the reply is sent |

```bash
# Reply after 500 ms
grpcurl -plaintext \
  -import-path ./proto -proto stayin_alive.proto \
  -d '{"delay_ms": 500}' \
  localhost:50051 stayin_alive.StayinAlive/PingMeLater
```

---

### Smoke-testing all endpoints

`scripts/test-endpoints.sh` exercises every endpoint and is run in CI.
You can run it locally against a live server:

```bash
./scripts/test-endpoints.sh
```

Optional environment variables: `HTTP_BASE` (default `http://localhost:3000`),
`GRPC_HOST` (default `localhost:50051`), `PROTO_PATH` (default `./proto`).
