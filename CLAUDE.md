# CLAUDE.md — chungustrator

## Overview

Container orchestrator for game sessions. Creates/destroys Docker containers for matches, manages port allocation, and communicates with chungusway via bidirectional gRPC streaming.

- **Language**: Rust (Axum + Tonic + Bollard)
- **Port**: 7000 (gRPC)
- **Status**: Active

## gRPC Service (port 7000)

### `CreateMatch` RPC
- **Input**: `MatchRequest { verification_codes: map<string, string> }` (player_id → code)
- **Output**: `MatchResponse { id, ip_address, lan_address, port }`
- Called by matchmaking service

### Bidirectional Streaming with Chungusway (connects to `127.0.0.1:50051`)

**Sends:** `VerificationCodeRequest` (codes to forward to game server), `Ping` (5s heartbeat)
**Receives:** `VerificationCodeResponse`, `GameServerShutdown` (container ID + reason), `Pong`

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `WAN_IP` | Public IP returned in CreateMatch response |
| `LAN_IP` | LAN IP returned in CreateMatch response |
| `SECRET_CHUNGUS` | JWT secret (passed to SQC containers) |
| `CHUNGUS_KEY` | API key (passed to SQC containers) |

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
cargo run             # binds 0.0.0.0:7000, connects to chungusway at 50051
cargo fmt && cargo clippy
```

## Architecture Notes

- **Port allocation**: Min-heap (`BinaryHeap<Reverse<u16>>`) starting at 28785, allocates in pairs (base + query port). Freed ports are recycled.
- **Container pairs**: Each match creates two Docker containers:
  1. `chungusmod:latest` — game server with port bindings
  2. `sqc:latest` — SQC sidecar (shared network namespace) — **legacy, planned for removal** once SQC is fully deprecated
- **Concurrency**: Tokio select loop over: 5s tick, service handler messages, chungusway inbound stream
- **Shutdown handling**: On `GameServerShutdown` from chungusway → stops container, releases ports
- No tests, no graceful shutdown for the orchestrator itself
