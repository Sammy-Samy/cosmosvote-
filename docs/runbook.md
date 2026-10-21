# CosmosVote Operational Runbook

Common operational tasks for administrators of the CosmosVote governance contract.

**Prerequisites** — all commands require these environment variables:

```bash
export GOVERNANCE_CONTRACT_ID=<your-governance-contract-id>
export STELLAR_SECRET_KEY=<your-admin-secret-key>
export STELLAR_RPC_URL=<rpc-url>               # e.g. https://soroban-testnet.stellar.org
export NETWORK_PASSPHRASE="Test SDF Network ; September 2015"  # or mainnet passphrase
```

---

## 1. Pause the Contract

Stops all state-changing operations (create proposal, cast vote, finalize, execute, cancel). Use during incidents or upgrades.

```bash
stellar contract invoke \
  --id "$GOVERNANCE_CONTRACT_ID" \
  --source "$STELLAR_SECRET_KEY" \
  --rpc-url "$STELLAR_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- pause \
  --admin "$(stellar keys address "$STELLAR_SECRET_KEY")"
```

**Verify:** any non-read call should return error `40 ContractPaused`.

---

## 2. Unpause the Contract

Resumes normal operations after a pause.

```bash
stellar contract invoke \
  --id "$GOVERNANCE_CONTRACT_ID" \
  --source "$STELLAR_SECRET_KEY" \
  --rpc-url "$STELLAR_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- unpause \
  --admin "$(stellar keys address "$STELLAR_SECRET_KEY")"
```

---

## 3. Update Quorum on an Active Proposal

Adjusts the quorum threshold on a proposal that is still **Active and has received zero votes**.

```bash
stellar contract invoke \
  --id "$GOVERNANCE_CONTRACT_ID" \
  --source "$STELLAR_SECRET_KEY" \
  --rpc-url "$STELLAR_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- update_quorum \
  --admin "$(stellar keys address "$STELLAR_SECRET_KEY")" \
  --proposal_id <PROPOSAL_ID> \
  --new_quorum <NEW_QUORUM>
```

Replace `<PROPOSAL_ID>` with the target proposal's numeric ID and `<NEW_QUORUM>` with the new minimum vote-weight threshold (must be > 0 and ≤ total token supply).

---

## 4. Transfer Admin (Two-Step)

Admin transfer is a two-step process to prevent accidental loss of admin access.

**Step 1 — current admin initiates the transfer:**

```bash
stellar contract invoke \
  --id "$GOVERNANCE_CONTRACT_ID" \
  --source "$STELLAR_SECRET_KEY" \
  --rpc-url "$STELLAR_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- transfer_admin \
  --admin "$(stellar keys address "$STELLAR_SECRET_KEY")" \
  --new_admin <NEW_ADMIN_ADDRESS>
```

**Step 2 — new admin accepts** (signed by the new admin's key):

```bash
stellar contract invoke \
  --id "$GOVERNANCE_CONTRACT_ID" \
  --source "$NEW_ADMIN_SECRET_KEY" \
  --rpc-url "$STELLAR_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- accept_admin \
  --pending_admin <NEW_ADMIN_ADDRESS>
```

The transfer is only complete after Step 2. Until then, the original admin retains control.

---

## 5. Deploy a New Contract Version

### 5a. In-Place WASM Upgrade (recommended for preserving contract ID)

Upgrades the contract bytecode in-place, keeping the same contract ID and all on-chain state. Use this when the storage schema is unchanged or only additive.

```bash
# 1. Build the new WASM
make build

# 2. Upload the new WASM to the network and capture the hash
NEW_WASM_HASH=$(stellar contract upload \
  --wasm target/wasm32-unknown-unknown/release/cosmosvote_governance.wasm \
  --source "$STELLAR_SECRET_KEY" \
  --rpc-url "$STELLAR_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" | jq -r '.hash')

echo "New WASM hash: $NEW_WASM_HASH"

# 3. (Optional but recommended) Pause before upgrading
stellar contract invoke \
  --id "$GOVERNANCE_CONTRACT_ID" \
  --source "$STELLAR_SECRET_KEY" \
  --rpc-url "$STELLAR_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- pause \
  --admin "$(stellar keys address "$STELLAR_SECRET_KEY")"

# 4. Invoke the upgrade entrypoint (admin only)
stellar contract invoke \
  --id "$GOVERNANCE_CONTRACT_ID" \
  --source "$STELLAR_SECRET_KEY" \
  --rpc-url "$STELLAR_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- upgrade \
  --admin "$(stellar keys address "$STELLAR_SECRET_KEY")" \
  --new_wasm_hash "$NEW_WASM_HASH"

# 5. Verify state is preserved
stellar contract invoke \
  --id "$GOVERNANCE_CONTRACT_ID" \
  --rpc-url "$STELLAR_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- get_config

stellar contract invoke \
  --id "$GOVERNANCE_CONTRACT_ID" \
  --rpc-url "$STELLAR_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- proposal_count

# 6. Unpause (if paused in step 3)
stellar contract invoke \
  --id "$GOVERNANCE_CONTRACT_ID" \
  --source "$STELLAR_SECRET_KEY" \
  --rpc-url "$STELLAR_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- unpause \
  --admin "$(stellar keys address "$STELLAR_SECRET_KEY")"
```

### 5b. Fresh Deploy (new contract ID)

```bash
# 1. Build the updated WASM
cargo build --release --target wasm32-unknown-unknown

# 2. Deploy new token contract (if changed)
NEW_TOKEN_ID=$(stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/cosmosvote_token.wasm \
  --source "$STELLAR_SECRET_KEY" \
  --rpc-url "$STELLAR_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE")

# 3. Deploy new governance contract
NEW_GOV_ID=$(stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/cosmosvote_governance.wasm \
  --source "$STELLAR_SECRET_KEY" \
  --rpc-url "$STELLAR_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE")

# 4. Initialize governance
ADMIN_ADDRESS=$(stellar keys address "$STELLAR_SECRET_KEY")
stellar contract invoke \
  --id "$NEW_GOV_ID" \
  --source "$STELLAR_SECRET_KEY" \
  --rpc-url "$STELLAR_RPC_URL" \
  --network-passphrase "$NETWORK_PASSPHRASE" \
  -- initialize \
  --admin "$ADMIN_ADDRESS" \
  --voting_token "$NEW_TOKEN_ID" \
  --min_proposal_balance 0 \
  --proposal_cooldown 0 \
  --restrict_admin_vote false

echo "New governance contract: $NEW_GOV_ID"
```

Update `GOVERNANCE_CONTRACT_ID` and `TOKEN_CONTRACT_ID` in your `.env` file after deployment. Pause the old contract before cutting over.

---

## Troubleshooting

| Error | Code | Cause | Fix |
|-------|------|-------|-----|
| `ContractPaused` | 40 | Contract is paused | Run `unpause` first |
| `NotPaused` | 41 | Calling `unpause` on an already-active contract | No action needed |
| `NotAdmin` | 30 | `--source` key is not the current admin | Use the correct admin key |
| `NoPendingAdmin` | — | Calling `accept_admin` with no pending transfer | Run `transfer_admin` first |
| `NotPendingAdmin` | — | `accept_admin` caller does not match pending admin | Use the exact address set in `transfer_admin` |
| `ProposalNotFound` | 10 | Invalid `--proposal_id` | Verify the proposal ID exists |
| `ProposalNotActive` | 11 | `update_quorum` called on a non-Active proposal | Only active proposals can be updated |
| `QuorumUpdateNotAllowed` | — | Proposal already has votes | Quorum can only be changed before any votes are cast |
| `QuorumExceedsSupply` | 16 | `new_quorum` > total token supply | Lower the quorum value |
| `AlreadyInitialized` | 1 | `initialize` called on an already-initialized contract | Deploy a fresh contract instead |

For the full error reference see [docs/errors.md](./errors.md).

---

## Post-Incident Procedure

After any incident that causes user-visible impact or requires an emergency response, open a postmortem within 48 hours of resolution.

1. Copy the template: `cp docs/postmortem-template.md docs/postmortems/YYYY-MM-DD-<slug>.md`
2. Fill in all sections (timeline, root cause, impact, action items, lessons learned)
3. Open a PR for team review
4. Track action items to completion

See [docs/postmortem-template.md](./postmortem-template.md) for the full template and guidance.
For a completed example (hypothetical RPC outage), see [docs/postmortems/2024-03-15-rpc-outage.md](./postmortems/2024-03-15-rpc-outage.md).

---

## Deployment Procedure

Use this section when deploying CosmosVote contracts to testnet or mainnet for the first time, or when upgrading to a new version.

### Pre-Flight Checks

Complete every item before running any deployment script.

- [ ] **Environment variables set** — all required vars are exported:
  ```bash
  echo "$GOVERNANCE_CONTRACT_ID" "$TOKEN_CONTRACT_ID" "$STELLAR_SECRET_KEY" \
       "$STELLAR_RPC_URL" "$NETWORK_PASSPHRASE"
  ```
  None of the above should be blank.
- [ ] **Tests passing** — the full test suite is green:
  ```bash
  make test
  ```
- [ ] **Backup taken** — export the current contract state (proposal IDs, quorum settings, admin address) and save it off-chain before any upgrade:
  ```bash
  stellar contract invoke --id "$GOVERNANCE_CONTRACT_ID" \
    --rpc-url "$STELLAR_RPC_URL" \
    --network-passphrase "$NETWORK_PASSPHRASE" \
    -- get_config
  ```
- [ ] **Quorum config verified** — confirm `min_quorum_bps` and per-proposal quorums are correct for this network (testnet vs. mainnet values differ).
- [ ] **Old contract paused** (upgrades only) — pause the existing contract before deploying the replacement (see [Section 1](#1-pause-the-contract)).

### Step-by-Step Deployment

1. **Build WASM binaries:**
   ```bash
   make build
   # Artifacts: target/wasm32-unknown-unknown/release/cosmosvote_governance.wasm
   #            target/wasm32-unknown-unknown/release/cosmosvote_token.wasm
   ```

2. **Run the deployment script:**
   - Testnet / local:
     ```bash
     bash scripts/deploy.sh
     ```
   - Mainnet:
     ```bash
     bash scripts/deploy_mainnet.sh
     ```
   Both scripts print the new `CONTRACT_ID` values on success.

3. **Record the new contract IDs** returned by the script and update `.env`:
   ```bash
   GOVERNANCE_CONTRACT_ID=<new-gov-id>
   TOKEN_CONTRACT_ID=<new-token-id>
   ```

4. **Initialize the governance contract** (fresh deploy only — skip for upgrades that reuse an existing instance):
   ```bash
   stellar contract invoke \
     --id "$GOVERNANCE_CONTRACT_ID" \
     --source "$STELLAR_SECRET_KEY" \
     --rpc-url "$STELLAR_RPC_URL" \
     --network-passphrase "$NETWORK_PASSPHRASE" \
     -- initialize \
     --admin "$(stellar keys address "$STELLAR_SECRET_KEY")" \
     --voting_token "$TOKEN_CONTRACT_ID" \
     --min_proposal_balance 0 \
     --proposal_cooldown 0 \
     --min_quorum_bps <BPS> \
     --restrict_admin_vote false
   ```

5. **Unpause** (upgrades only — if old contract was paused in pre-flight):
   ```bash
   # Unpause the NEW contract if it was deployed in paused state
   # (fresh deploys are unpaused by default)
   ```

6. **Verify WASM build integrity** using the test script:
   ```bash
   bash scripts/test_wasm.sh
   ```

### Post-Flight Checks

Perform these checks immediately after deployment completes.

- [ ] **Health check** — confirm the contract responds to a read call:
  ```bash
  stellar contract invoke \
    --id "$GOVERNANCE_CONTRACT_ID" \
    --rpc-url "$STELLAR_RPC_URL" \
    --network-passphrase "$NETWORK_PASSPHRASE" \
    -- get_config
  ```
  Expected: returns the config struct without error.
- [ ] **Event emission test** — create a test proposal and verify an `proposal_created` event appears in the Horizon event stream:
  ```bash
  stellar events --network testnet \
    --contract-id "$GOVERNANCE_CONTRACT_ID" \
    --start-ledger <DEPLOY_LEDGER>
  ```
- [ ] **Active proposals still accessible** (upgrades only) — query each known active proposal ID and confirm status is still `Active`:
  ```bash
  stellar contract invoke \
    --id "$GOVERNANCE_CONTRACT_ID" \
    --rpc-url "$STELLAR_RPC_URL" \
    --network-passphrase "$NETWORK_PASSPHRASE" \
    -- get_proposal --proposal_id <ID>
  ```
- [ ] **Frontend connected** — update `VITE_GOVERNANCE_CONTRACT_ID` and `VITE_TOKEN_CONTRACT_ID` in `frontend/.env`, restart the dev server, and confirm proposals load.
- [ ] **Notification service updated** — restart the notification service with the new contract ID so it resumes event polling from the correct ledger.
