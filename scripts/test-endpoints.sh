#!/usr/bin/env bash
# Smoke-test every HTTP and gRPC endpoint.
#
# Environment variables (all optional):
#   HTTP_BASE   Base URL of the HTTP server  (default: http://localhost:3000)
#   GRPC_HOST   Host:port of the gRPC server (default: localhost:50051)
#   PROTO_PATH  Directory containing stayin_alive.proto
#               (default: <repo-root>/proto, resolved relative to this script)
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

HTTP_BASE="${HTTP_BASE:-http://localhost:3000}"
GRPC_HOST="${GRPC_HOST:-localhost:50051}"
PROTO_PATH="${PROTO_PATH:-${script_dir}/../proto}"

pass() { echo "PASS"; }
fail() { echo "FAIL: $*"; exit 1; }

# ---------------------------------------------------------------------------
# HTTP /ping
# ---------------------------------------------------------------------------
echo "==> GET /ping"
response=$(curl -sf "${HTTP_BASE}/ping")
[ "${response}" = "pong" ] || fail "expected 'pong', got '${response}'"
pass

# ---------------------------------------------------------------------------
# HTTP /sse-ping  — verify Content-Type: text/event-stream
# ---------------------------------------------------------------------------
echo "==> GET /sse-ping"
headers=$(curl -sD - --max-time 2 -o /dev/null \
  "${HTTP_BASE}/sse-ping?interval_ms=500" 2>/dev/null || true)
echo "${headers}" | grep -qi "content-type: text/event-stream" \
  || fail "expected Content-Type: text/event-stream"
pass

# ---------------------------------------------------------------------------
# HTTP /ws  — verify WebSocket upgrade (HTTP 101)
# ---------------------------------------------------------------------------
echo "==> GET /ws (WebSocket upgrade)"
http_code=$(curl -so /dev/null -w "%{http_code}" --max-time 2 \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==" \
  -H "Sec-WebSocket-Version: 13" \
  "${HTTP_BASE}/ws" 2>/dev/null || true)
[ "${http_code}" = "101" ] || fail "expected HTTP 101, got '${http_code}'"
pass

# ---------------------------------------------------------------------------
# gRPC Ping
# ---------------------------------------------------------------------------
echo "==> gRPC stayin_alive.StayinAlive/Ping"
grpcurl -plaintext \
  -import-path "${PROTO_PATH}" -proto stayin_alive.proto \
  "${GRPC_HOST}" stayin_alive.StayinAlive/Ping
pass

# ---------------------------------------------------------------------------
# gRPC PingMeLater
# ---------------------------------------------------------------------------
echo "==> gRPC stayin_alive.StayinAlive/PingMeLater"
grpcurl -plaintext \
  -import-path "${PROTO_PATH}" -proto stayin_alive.proto \
  -d '{"delay_ms": 100}' \
  "${GRPC_HOST}" stayin_alive.StayinAlive/PingMeLater
pass

echo "All endpoint tests passed."
