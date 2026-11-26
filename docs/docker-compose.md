# Docker Compose Local Dev Environment

This guide covers running the full CosmosVote stack locally using Docker Compose.

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/) 24+
- [Docker Compose](https://docs.docker.com/compose/install/) v2 (bundled with Docker Desktop)

## Services

| Service | Description | Port |
|---------|-------------|------|
| `dev` | Builder image — full Rust toolchain for development and testing | — |
| `stellar-node` | Local Stellar node with Soroban RPC enabled | `8001:8000` |
| `rpc-proxy` | Nginx reverse proxy in front of the Stellar RPC | `8000:8000` |
| `faucet` | Faucet service — signs and submits testnet funding transactions | `3000:3000` |
| `artifacts` | Minimal runtime image shipping WASM artifacts only (profile: `artifacts`) | — |

## Networks

Services no longer share a single default network. Three custom networks isolate
traffic so that each service can only talk to the peers it is supposed to:

| Network | Purpose | Attached services |
|---------|---------|-------------------|
| `frontend-net` | Browser-facing edge network | `frontend`, `rpc-proxy` |
| `backend-net` | Application/backend network | `faucet`, `rpc-proxy` |
| `stellar-net` | Internal Stellar node network | `stellar-node`, `rpc-proxy`, `dev` |

`rpc-proxy` is the only service attached to more than one network and acts as
the sole bridge between the frontend, the faucet backend and the Stellar node.

```
                 ┌──────────────┐
                 │   frontend   │
                 └──────┬───────┘
                        │ frontend-net
                 ┌──────┴───────┐
                 │  rpc-proxy   │
                 └──┬────────┬──┘
          backend-net│        │stellar-net
              ┌──────┴──┐  ┌──┴───────────┐
              │ faucet  │  │ stellar-node │
              └─────────┘  └──────────────┘
```

### Access rules

- **Faucet → Stellar node:** the `faucet` service is *not* attached to
  `stellar-net`, so it cannot reach `stellar-node:8000` directly. All faucet
  traffic to the node must go through `rpc-proxy` (`http://rpc-proxy:8000`),
  which keeps rate limiting and request logging in effect.
- **Frontend → faucet backend:** the `frontend` service is *not* attached to
  `backend-net`, so it cannot reach the faucet backend directly. Frontend
  requests are routed through `rpc-proxy`.
- **Frontend → Stellar node:** the `frontend` service is not attached to
  `stellar-net`, so `stellar-node` is unreachable from the frontend.
- **Dev shell:** the `dev` service joins `stellar-net` only, so it can reach
  the node for deployment scripts but not the faucet or frontend.

> If you add a new service, attach it to the narrowest network that satisfies
> its needs. Do not attach application services to `stellar-net` unless they
> must talk to the node directly.

## Environment Variables

Copy the example file and edit as needed:

```bash
cp .env.example .env
```

Key variables used by Docker Compose:

| Variable | Default | Description |
|----------|---------|-------------|
| `NETWORK` | `local` | Network to connect to (`local` \| `testnet` \| `mainnet`) |
| `STELLAR_RPC_URL` | `http://rpc-proxy:8000` | Soroban RPC endpoint (via `rpc-proxy`; services must not target `stellar-node` directly) |
| `STELLAR_NETWORK_PASSPHRASE` | `Standalone Network ; February 2021` | Must match the running node |
| `STELLAR_SECRET_KEY` | — | Admin secret key for deployment scripts |
| `FAUCET_SECRET_KEY_FILE` | `/run/secrets/faucet_secret_key` | Path to the file containing the faucet signing keypair (mounted via Compose `secrets:`) |
| `GOVERNANCE_CONTRACT_ID` | — | Populated after running `deploy.sh` |
| `TOKEN_CONTRACT_ID` | — | Populated after running `deploy.sh` |

> The `dev` service injects `STELLAR_RPC_URL` and `STELLAR_NETWORK_PASSPHRASE` automatically. You do not need to set them in `.env` for local development.

### Faucet secret

The faucet signing keypair is **not** passed via an environment variable. Instead, the `faucet` service reads it from a file whose path is given by `FAUCET_SECRET_KEY_FILE`. Docker Compose mounts the secret read-only at `/run/secrets/faucet_secret_key`:

```yaml
services:
  faucet:
    environment:
      FAUCET_SECRET_KEY_FILE: /run/secrets/faucet_secret_key
    secrets:
      - faucet_secret_key

secrets:
  faucet_secret_key:
    file: ./secrets/faucet_secret_key.txt
```

Create the secret file locally (the `secrets/` directory is git-ignored — never commit it):

```bash
mkdir -p secrets
printf '%s' 'SXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX' > secrets/faucet_secret_key.txt
chmod 600 secrets/faucet_secret_key.txt
```

The file is mounted read-only at `/run/secrets/faucet_secret_key` inside the container, so it is never exposed through `/proc/<pid>/environ`, logs, or crash dumps. The key is never logged, even at `DEBUG` level.

> **Deprecated:** the `FAUCET_ADMIN_SECRET_KEY` environment variable is still accepted as a fallback for local development only and emits a deprecation warning at startup. Do not use it — prefer `FAUCET_SECRET_KEY_FILE` (or AWS Secrets Manager in production).

## Quick Start

### 1. Start the local Stellar node

```bash
docker compose up stellar-node -d
```

The node exposes the Soroban RPC at `http://localhost:8001`. The `rpc-proxy` re-exposes it at `http://localhost:8000`.

### 2. Run tests inside the dev container

```bash
docker compose run --rm dev make test
```

### 3. Build WASM binaries

```bash
docker compose run --rm dev make build
```

Built WASM artifacts are written to `target/wasm32-unknown-unknown/release/` inside the mounted workspace volume.

### 4. Open an interactive dev shell

```bash
docker compose run --rm dev bash
```

From inside the shell you have access to the full Rust toolchain, `stellar` CLI, `make`, and all project scripts.

### 5. Build the minimal runtime (WASM artifacts only)

```bash
docker compose --profile artifacts build artifacts
```

This produces a small `debian:bookworm-slim`-based image containing only the compiled WASM files and the Stellar CLI — suitable for CI artifact storage or deployment pipelines.

## Full Stack (node + dev shell)

```bash
# Bring up the Stellar node and proxy in the background
docker compose up stellar-node rpc-proxy -d

# Run the deploy script against the local node
docker compose run --rm dev bash scripts/deploy.sh

# Watch logs
docker compose logs -f stellar-node
```

## Stopping / Cleaning Up

```bash
# Stop all services
docker compose down

# Remove volumes (clears the Stellar chain state and Cargo cache)
docker compose down -v
```

## Ports Summary

| Host port | Container port | Service |
|-----------|---------------|---------|
| `8000` | `8000` | `rpc-proxy` (Soroban RPC) |
| `8001` | `8000` | `stellar-node` (direct access) |

> If port `8000` is already in use on your machine, edit `docker-compose.yml` and change the host-side port mapping for `rpc-proxy`.

## Troubleshooting

**`stellar-node` health check fails on first start**  
The Stellar quickstart image takes up to 30 seconds to initialize. The `dev` service waits for it automatically via `depends_on: condition: service_healthy`. If the health check never passes, check the node logs:

```bash
docker compose logs stellar-node
```

**Permission errors on the mounted workspace volume**  
The container runs as root. If generated files (e.g., `target/`) are owned by root on the host, run:

```bash
sudo chown -R $USER:$USER target/
```

**Cargo cache not persisting between runs**  
The `cargo-cache` and `target-cache` named volumes persist between `docker compose run` invocations. They are cleared only when you run `docker compose down -v`.

**Service cannot resolve a hostname (e.g. `faucet` cannot reach `stellar-node`)**  
This is expected: services are isolated on separate networks (see [Networks](#networks)). Use `rpc-proxy` as the entry point instead of connecting to `stellar-node` directly. If a service genuinely needs direct access, attach it to the relevant network in `docker-compose.yml` and update this document.

**`docker compose up` fails with "network ... not found"**  
Stale networks from an older Compose file may linger. Recreate them:

```bash
docker compose down
docker compose up -d
```

**Frontend requests to the faucet time out**  
The frontend cannot reach the faucet backend directly. Confirm requests are routed through `rpc-proxy` and that `rpc-proxy` is attached to both `frontend-net` and `backend-net`:

```bash
docker compose exec rpc-proxy sh -c 'getent hosts faucet'
```
