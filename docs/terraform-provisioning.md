# Infrastructure Provisioning Guide

This guide covers how to initialize and apply infrastructure for each CosmosVote environment using the provided configuration files and deploy scripts.

## Environments

CosmosVote supports three deployment environments:

| Environment | Config File | Network |
|-------------|-------------|--------|
| Local | `config/local.toml` | Standalone (localhost:8000) |
| Testnet | `config/testnet.toml` | Test SDF Network |
| Mainnet | `config/mainnet.toml` | Public Global Stellar Network |

## Prerequisites

- [Stellar CLI](https://developers.stellar.org/docs/tools/developer-tools/cli/install-stellar-cli) installed
- Rust with `wasm32-unknown-unknown` target
- A funded Stellar account (testnet/mainnet)
- `.env` file configured (see `.env.example`)

```bash
cp .env.example .env
# Edit .env with your STELLAR_SECRET_KEY, network settings, etc.
```

## Initializing Each Environment

### Local

```bash
# Start a local Stellar node (requires Docker)
docker compose up -d

# Deploy contracts to local network
NETWORK=local bash scripts/deploy.sh
```

### Testnet

```bash
# Fund your account on testnet
stellar account fund --network testnet <YOUR_PUBLIC_KEY>

# Deploy to testnet
NETWORK=testnet bash scripts/deploy.sh
```

### Mainnet

```bash
# Ensure your account is funded with real XLM
# Review scripts/deploy_mainnet.sh carefully before running
NETWORK=mainnet bash scripts/deploy_mainnet.sh
```

## Variable Structure

Each environment config (`config/*.toml`) provides:

```toml
rpc_url = "<Soroban RPC endpoint>"
network_passphrase = "<Network identifier string>"
network = "<local|testnet|mainnet>"
```

### Environment-specific values

**local.toml**
```toml
rpc_url = "http://localhost:8000"
network_passphrase = "Standalone Network ; February 2021"
network = "local"
```

**testnet.toml**
```toml
rpc_url = "https://soroban-testnet.stellar.org"
network_passphrase = "Test SDF Network ; September 2015"
network = "testnet"
```

**mainnet.toml**
```toml
rpc_url = "https://soroban-mainnet.stellar.org"
network_passphrase = "Public Global Stellar Network ; September 2015"
network = "mainnet"
```

Additional variables are set via `.env`:

| Variable | Description |
|----------|-------------|
| `STELLAR_SECRET_KEY` | Deployer account secret key |
| `STELLAR_RPC_URL` | Overrides config RPC URL if set |
| `GOVERNANCE_CONTRACT_ID` | Deployed governance contract address |
| `TOKEN_CONTRACT_ID` | Deployed token contract address |
| `NETWORK` | Active environment (`local`/`testnet`/`mainnet`) |

## Cost Estimation (Infracost)

Pull requests that touch `infra/terraform/` automatically run [Infracost](https://www.infracost.io/) to estimate the cost impact of the change.

### How it works

1. The Terraform CI workflow runs `infracost breakdown` on the base branch and on the PR branch.
2. `infracost diff` computes the monthly cost delta.
3. The diff is posted as a comment on the PR, so reviewers can see cost changes alongside the code.

### Approval threshold

If the estimated monthly cost increase exceeds **$0/month**, the PR requires manual approval from a maintainer before it can be merged. Review the cost diff comment and approve the workflow run (or the PR) only after confirming the increase is intentional.

### Setting up `INFRACOST_API_KEY`

The workflow requires an Infracost API key, stored as a repository secret:

1. Sign up at [dashboard.infracost.io](https://dashboard.infracost.io/) and generate an API key.
2. In GitHub, go to **Settings → Secrets and variables → Actions**.
3. Add a new repository secret named `INFRACOST_API_KEY` with the generated key.

### Running locally

To estimate costs before opening a PR:

```bash
# Install the CLI (see https://www.infracost.io/docs/#quick-start)
# then authenticate once:
infracost auth login

# Show the cost breakdown for the Terraform configuration
infracost breakdown --path infra/terraform

# Compare against a saved baseline to see the diff
infracost breakdown --path infra/terraform --format json --out-file infracost-base.json
# (make your changes, then)
infracost diff --path infra/terraform --compare-to infracost-base.json
```

## Troubleshooting

### `Error: account not found`
The deployer account is not funded. Fund it via `stellar account fund` (testnet) or a wallet (mainnet).

### `Error: contract already initialized`
The contract was previously deployed to this network. To redeploy, use a new account or update the contract ID in `.env`.

### `Error: WASM binary not found`
Build the contracts first:
```bash
make build
```

### RPC connection refused (local)
Ensure the local Docker node is running:
```bash
docker compose up -d
docker compose ps  # verify 'stellar' service is healthy
```

### Transaction simulation failed
Check that your `network_passphrase` in the config matches the running network exactly.
