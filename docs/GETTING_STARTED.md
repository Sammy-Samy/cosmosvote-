# Getting Started with CosmosVote

This guide walks you through setting up CosmosVote from scratch.

## Prerequisites

CosmosVote builds on macOS, Linux, and Windows (via WSL2). Install the tools below for your platform.

| Tool | macOS | Linux | Windows / WSL2 |
|------|-------|-------|----------------|
| Rust toolchain | 1.75+ via [rustup](https://rustup.rs) (do **not** use `brew install rust`) | 1.75+ via [rustup](https://rustup.rs) | 1.75+ via rustup **inside WSL2** |
| Node.js | 20 or 22 LTS (via `nvm` or Homebrew) | 20 or 22 LTS (via `nvm` or distro package) | 20 or 22 LTS installed **inside WSL2** |
| Docker | Docker Desktop 24+ with Compose v2 | Docker Engine 24+ with Compose v2 plugin | Docker Desktop 24+ with **WSL2 backend** enabled |
| Stellar CLI | 22+ | 22+ | 22+ inside WSL2 |
| Platform tools | Homebrew, Xcode Command Line Tools (`xcode-select --install`) | `build-essential`, `pkg-config`, `libssl-dev` | WSL2 (Ubuntu 22.04+ recommended), `build-essential`, `pkg-config`, `libssl-dev` |

> **Version pinning:** the repo pins the Rust toolchain via `rust-toolchain.toml` (and `Cargo.toml` `rust-version`). rustup will automatically select the correct toolchain when you `cd` into the repo — this is why rustup is required over distro/Homebrew Rust packages.

## Step 1 — Clone & Build

```bash
git clone https://github.com/mercyJJ/cosmosvote.git
cd cosmosvote
rustup target add wasm32-unknown-unknown
make build
```

## Step 2 — Run Tests

```bash
make test
```

All 60+ tests should pass.

## Step 3 — Configure Environment

```bash
cp .env.example .env
# Edit .env: set STELLAR_SECRET_KEY and NETWORK
```

## Step 4 — Deploy to Testnet

```bash
NETWORK=testnet ./scripts/deploy.sh
```

Copy the printed contract IDs into your `.env`.

## Step 5 — Interact with Contracts

```bash
# Create a proposal
stellar contract invoke \
  --id $GOVERNANCE_CONTRACT_ID \
  --source $STELLAR_SECRET_KEY \
  --network testnet \
  -- create_proposal \
  --proposer <YOUR_ADDRESS> \
  --title "My First Proposal" \
  --description "A test governance proposal" \
  --quorum 1000000 \
  --duration 3600

# Cast a vote
stellar contract invoke \
  --id $GOVERNANCE_CONTRACT_ID \
  --source $STELLAR_SECRET_KEY \
  --network testnet \
  -- cast_vote \
  --voter <YOUR_ADDRESS> \
  --proposal_id 0 \
  --vote '{"tag":"Yes"}'
```

## Step 6 — Run the Frontend

The frontend is a React + Vite application that allows you to browse proposals.

```bash
cd frontend
# 1. Install dependencies
npm install

# 2. Configure environment
cp .env.example .env
# Edit .env and set:
# VITE_GOVERNANCE_CONTRACT_ID="CB..."
# VITE_TOKEN_CONTRACT_ID="CD..."
# VITE_RPC_URL="http://localhost:8000" (or testnet URL)

# 3. Start development server
npm run dev
# Open http://localhost:5173
```

### Frontend Configuration

| Variable | Description |
|----------|-------------|
| `VITE_GOVERNANCE_CONTRACT_ID` | The ID of the deployed governance contract |
| `VITE_TOKEN_CONTRACT_ID` | The ID of the deployed token contract |
| `VITE_RPC_URL` | The Soroban RPC endpoint (local, testnet, or mainnet) |
| `VITE_NETWORK_PASSPHRASE` | Network passphrase for signing transactions |


## Platform-Specific Setup

### macOS

1. **Install rustup, not Homebrew Rust.** Homebrew's `rust` formula often lags behind the pinned toolchain and does not respect `rust-toolchain.toml`.
   ```bash
   # If you previously installed Rust via Homebrew, remove it first:
   brew uninstall rust
   # Then install rustup:
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   # Ensure rustup shims take precedence in PATH:
   source "$HOME/.cargo/env"
   which rustc   # should print ~/.cargo/bin/rustc, NOT /opt/homebrew/bin/rustc
   ```
2. **Toolchain pinning.** `rust-toolchain.toml` selects the pinned toolchain automatically. Verify with `rustc --version` inside the repo — it should match the pin, not the Homebrew version.
3. **Apple Silicon (aarch64).** All tooling (rustup, Node, Docker Desktop) ships native arm64 builds. If you see `linker 'cc' not found`, install Xcode Command Line Tools: `xcode-select --install`. Rosetta is **not** required.
4. **Docker networking.** Docker Desktop on macOS exposes the host via `host.docker.internal`; use this instead of `localhost` from inside containers.

### Linux

1. Install build dependencies: `sudo apt install build-essential pkg-config libssl-dev`.
2. Install rustup (see above) and Docker Engine 24+ with the Compose v2 plugin (`docker compose version` should report v2.x).
3. Add your user to the `docker` group (`sudo usermod -aG docker $USER`) and re-login, or prefix commands with `sudo`.

### Windows / WSL2

1. **Install WSL2** (PowerShell as Administrator):
   ```powershell
   wsl --install -d Ubuntu-22.04
   wsl --set-default-version 2
   ```
2. **Docker Desktop with WSL2 backend.** In Docker Desktop → Settings → General, enable *Use the WSL 2 based engine*; under *Resources → WSL Integration*, enable integration for your Ubuntu distro. Verify inside WSL: `docker compose version`.
3. **Keep the repo inside the WSL filesystem**, e.g. `~/cosmosvote`. Cloning under `/mnt/c/...` causes severe I/O slowdowns and permission issues with `target/`.
   ```bash
   cd ~ && git clone https://github.com/mercyJJ/cosmosvote.git
   ```
4. **Networking from Windows to containers.** From Windows, reach services running in WSL2 containers via `localhost` (WSL2 forwards ports). From *inside* a container reaching a Stellar node running on the Windows host, use `host.docker.internal` instead of `localhost`.
5. **Line endings.** Configure Git to avoid CRLF in shell scripts:
   ```bash
   git config --global core.autocrlf input
   ```
   If scripts fail with `bad interpreter: /bin/bash^M`, run `dos2unix scripts/*.sh` or re-checkout with `git config core.autocrlf input`.

## Known Issues & Workarounds

| Symptom | Cause | Workaround |
|---------|-------|------------|
| `error: package requires rustc 1.75 or newer` but `rustc --version` shows an older version | Homebrew Rust shadowing rustup | `brew uninstall rust`, `source "$HOME/.cargo/env"`, confirm `which rustc` points to `~/.cargo/bin/rustc` |
| `rust-toolchain.toml` ignored / wrong toolchain selected | Non-rustup Rust in PATH | Reinstall via rustup; ensure `~/.cargo/bin` precedes `/opt/homebrew/bin` and `/usr/bin` in `PATH` |
| Container cannot reach Stellar node at `localhost:8000` | `localhost` inside a container refers to the container itself | Use `host.docker.internal:8000` (macOS/Windows) or the Docker bridge gateway IP (Linux) |
| Docker commands fail inside WSL2 (`Cannot connect to the Docker daemon`) | WSL integration disabled | Enable *Resources → WSL Integration* for your distro in Docker Desktop; restart the WSL distro (`wsl --shutdown`) |
| `bind: address already in use` on port 8000 | Another Stellar/Soroban node or dev server already running | `lsof -i :8000` (macOS/Linux) or `netstat -ano | findstr :8000` (Windows) and stop the process, or set a different port in `.env` |
| `Permission denied` writing to `target/` or files owned by `root` | Containers ran as root and created root-owned files | `sudo chown -R $USER:$USER target/` (or the whole repo); prefer running `docker compose run --rm dev ...` as documented |
| `bad interpreter: /bin/bash^M` when running scripts | CRLF line endings from Windows checkout | `git config --global core.autocrlf input` and re-checkout, or `dos2unix scripts/*.sh` |
| Very slow builds / file watcher issues on Windows | Repo cloned under `/mnt/c/...` | Move the clone into the WSL filesystem (`~/cosmosvote`) |

## Verification Checklist

Each platform must be verified by a developer before this guide is considered current.

| Platform | Verified by | Date | Notes |
|----------|-------------|------|-------|
| macOS (Apple Silicon) | _pending_ | — | rustup + Docker Desktop |
| Linux (Ubuntu 22.04) | _pending_ | — | Docker Engine + Compose v2 |
| Windows / WSL2 (Ubuntu 22.04) | _pending_ | — | Docker Desktop WSL2 backend |

## Related Documentation

- [Docker Compose setup](./docker-compose.md)
- [Local deployment](./local-deployment.md)
- [Local node testing](./local-node-testing.md)

## Next Steps

- Read the [Proposal Lifecycle](./lifecycle.md)
- Review the [Storage Model](./storage.md)
- Check the [Error Reference](./errors.md)
- Browse [Architecture Decision Records](./adr/)

## Promotion Workflow: local → testnet → mainnet

Changes must follow this promotion path before reaching mainnet:

```
local  →  testnet (staging)  →  mainnet
```

### Step 1 — Validate locally

```bash
docker compose up
docker compose run --rm dev make test
```

### Step 2 — Deploy to testnet (staging)

```bash
STELLAR_SECRET_KEY=<testnet-key> ./scripts/deploy_testnet.sh
```

Record the printed SHA-256 WASM hashes. The CI `testnet-gate` workflow runs this automatically on every push to `main`.

### Step 3 — Promote to mainnet

Only after testnet smoke tests pass:

```bash
# Dry-run first to preview all commands
./scripts/deploy_mainnet.sh --dry-run

# Deploy with hash verification
./scripts/deploy_mainnet.sh \
  --expected-token-hash <sha256-from-testnet> \
  --expected-gov-hash   <sha256-from-testnet>
```

Use `--yes` to skip the interactive prompt in automated pipelines.

### CI gate

The `.github/workflows/testnet-gate.yml` workflow deploys to testnet and runs smoke tests on every merge to `main`. Mainnet deployments are always manual and require the WASM hashes printed by the testnet deployment.
