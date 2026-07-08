# AGENTS.md — chungustrator

## Overview

Container orchestrator for game sessions. Creates/destroys Docker containers for matches, manages port allocation, and communicates with chungusway via bidirectional gRPC streaming.

- **Language**: Rust (Axum + Tonic + Bollard)
- **Port**: 7100 (gRPC; default, override with `CHUNGUSTRATOR_PORT`)
- **Status**: Active

## gRPC Service (port 7100)

### `CreateMatch` RPC
- **Input**: `MatchRequest { verification_codes: map<string, string> }` (player_id → code)
- **Output**: `MatchResponse { id, ip_address, lan_address, port }`
- Called by matchmaking service

### Bidirectional Streaming with Chungusway (dials `CHUNGUSWAY_URL`, default `http://127.0.0.1:50051`)

Startup retries the connection every 2s until chungusway is reachable — start order between the two no longer matters.

**Sends:** `VerificationCodeRequest` (codes to forward to game server), `Ping` (one initial ping when the stream is established — there is **no** periodic heartbeat)
**Receives:** `VerificationCodeResponse`, `GameServerShutdown` (container ID + reason), `Pong`

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `CHUNGUSTRATOR_PORT` | Port the gRPC server binds | `7100` |
| `CHUNGUSWAY_URL` | chungusway gRPC address to dial | `http://127.0.0.1:50051` |
| `WAN_IP` | Public IP returned in CreateMatch response | — |
| `LAN_IP` | LAN IP returned in CreateMatch response | — |
| `SECRET_CHUNGUS` | JWT secret (passed to SQC containers) | — |
| `CHUNGUS_KEY` | API key (passed to SQC containers); must equal one of auth's `CHUNGUS_API_KEY_*` values | — |

Port moved off 7000 because macOS's AirPlay Receiver (`ControlCenter`) squats it system-wide.

## Key Files

| File | Purpose |
|------|---------|
| `src/main.rs` | Entry point: gRPC server + chungusway client setup |
| `src/orchestrator.rs` | Core logic: ServerAllocator, Docker lifecycle, streaming |
| `src/service.rs` | gRPC service handler for CreateMatch |
| `src/handler.rs` | Axum HTTP handler stub (unused) |
| `proto/chungustrator.proto` | CreateMatch RPC definition |
| `proto/chungustrator_enet_streaming.proto` | Bidirectional streaming service |
| `.env.example` | Environment template |

## Development

```bash
cargo build
cargo run             # binds 0.0.0.0:7100, dials chungusway at CHUNGUSWAY_URL (retries until up)
cargo fmt && cargo clippy
```

## Architecture Notes

- **Port allocation**: Min-heap (`BinaryHeap<Reverse<u16>>`) starting at 28785, allocates in pairs (base + query port). **Caveat**: ports are only released on container-creation *failure* rollback — normal match teardown (`shutdown_container`) removes the server entry but never calls `release_port`, so `next_port` grows by 2 per match in steady state.
- **Container pairs**: Each match creates two Docker containers on the default bridge (with `host.docker.internal:host-gateway`):
  1. `chungusmod:latest` — game server with port bindings
  2. `sqc:latest` — SQC sidecar (shares the game container's network namespace) — **legacy, planned for removal** once SQC is fully deprecated. Gets `PLAYER_SERVICE_IP`/`AUTH_SERVICE_IP` pointing at `host.docker.internal` (compose-network hostnames don't resolve on the default bridge).
- **Concurrency**: Tokio select loop over: 5s tick, service handler messages, chungusway inbound stream
- **Shutdown handling**: On `GameServerShutdown` from chungusway → stops the game container (the SQC sidecar's id is never persisted, so it is likely leaked)
- No tests, no graceful shutdown for the orchestrator itself
