#!/bin/bash
# Deploy script with multisig admin support
# Usage: ./deploy_multisig.sh <network> [multisig_address]
#
# Demonstrates the two-of-three multisig treasury governance pattern:
#   - A 2-of-3 multisig account is created/configured (three signers, threshold 2)
#     unless an existing multisig address is supplied.
#   - The multisig becomes the governance admin (two-step transfer + accept).
#   - The treasury is initialized with the GOVERNANCE contract address as its
#     admin, so every disbursement flows through governance AND requires the
#     multisig to approve the resulting admin operation.
#
# See docs/admin-multisig-pattern.md for the full pattern description.

set -e

NETWORK=${1:-testnet}
MULTISIG_ADDRESS=${2}

# Multisig configuration: two-of-three (threshold 2, three signers).
MULTISIG_THRESHOLD=2
# Signer public keys (G... addresses). Override via environment variables.
SIGNER_1=${SIGNER_1:-}
SIGNER_2=${SIGNER_2:-}
SIGNER_3=${SIGNER_3:-}

echo "Deploying CosmosVote to $NETWORK with 2-of-3 multisig governance admin"

# Load config
CONFIG_FILE="config/${NETWORK}.toml"
if [ ! -f "$CONFIG_FILE" ]; then
    echo "Error: Config file not found: $CONFIG_FILE"
    exit 1
fi

# Step 1: Build contracts
echo "Building contracts..."
cd contracts/governance
cargo build --release --target wasm32-unknown-unknown
GOVERNANCE_WASM=target/wasm32-unknown-unknown/release/cosmosvote_governance.wasm

cd ../token
cargo build --release --target wasm32-unknown-unknown
TOKEN_WASM=target/wasm32-unknown-unknown/release/cosmosvote_token.wasm

cd ../treasury
cargo build --release --target wasm32-unknown-unknown
TREASURY_WASM=target/wasm32-unknown-unknown/release/cosmosvote_treasury.wasm

cd ../..

echo "✓ Contracts built"

# Step 1b: Create/configure the two-of-three multisig account
if [ -z "$MULTISIG_ADDRESS" ]; then
    if [ -z "$SIGNER_1" ] || [ -z "$SIGNER_2" ] || [ -z "$SIGNER_3" ]; then
        echo "Error: No multisig address supplied and SIGNER_1/SIGNER_2/SIGNER_3 not set."
        echo "Usage: $0 <network> <multisig_address>"
        echo "   or: SIGNER_1=G... SIGNER_2=G... SIGNER_3=G... $0 <network>"
        exit 1
    fi

    echo "Creating 2-of-3 multisig account (threshold=$MULTISIG_THRESHOLD)..."
    MULTISIG_ADDRESS=$(stellar keys generate \
        --network "$NETWORK" \
        --multisig \
        --threshold "$MULTISIG_THRESHOLD" \
        --signer "$SIGNER_1" \
        --signer "$SIGNER_2" \
        --signer "$SIGNER_3" \
        | grep "Public Key" | awk '{print $NF}')

    echo "✓ Multisig account created: $MULTISIG_ADDRESS"
else
    echo "Using existing multisig account: $MULTISIG_ADDRESS"
fi

echo "Multisig policy: 2-of-3 (any two of the three signers must approve)"

# Step 2: Deploy token contract
echo "Deploying token contract..."
TOKEN_ID=$(stellar contract deploy \
    --network "$NETWORK" \
    --source-account default \
    "$TOKEN_WASM" \
    | grep "Contract ID" | awk '{print $NF}')

echo "✓ Token contract deployed: $TOKEN_ID"

# Step 3: Initialize token with initial admin (will be transferred later)
# For multisig transition, use a temporary EOA as initial admin
TEMP_ADMIN=$(stellar account create default --network "$NETWORK" | grep "Account ID" | awk '{print $NF}')

echo "Initializing token..."
stellar contract invoke \
    --network "$NETWORK" \
    --source-account default \
    --id "$TOKEN_ID" \
    -- initialize \
    --admin "$TEMP_ADMIN" \
    --initial_supply 1000000000

echo "✓ Token initialized"

# Step 4: Deploy governance contract
echo "Deploying governance contract..."
GOVERNANCE_ID=$(stellar contract deploy \
    --network "$NETWORK" \
    --source-account default \
    "$GOVERNANCE_WASM" \
    | grep "Contract ID" | awk '{print $NF}')

echo "✓ Governance contract deployed: $GOVERNANCE_ID"

# Step 5: Initialize governance with temporary admin
echo "Initializing governance..."
stellar contract invoke \
    --network "$NETWORK" \
    --source-account default \
    --id "$GOVERNANCE_ID" \
    -- initialize \
    --admin "$TEMP_ADMIN" \
    --voting_token "$TOKEN_ID" \
    --min_proposal_balance 0 \
    --proposal_cooldown 0 \
    --min_quorum_bps 5000 \
    --restrict_admin_vote false

echo "✓ Governance contract initialized"

# Step 5b: Deploy and initialize the treasury contract.
# The treasury admin is the GOVERNANCE contract address (not the multisig).
# Disbursements therefore require a governance proposal to pass, and the
# resulting admin operation must additionally be approved by the 2-of-3
# multisig that controls the governance admin role.
echo "Deploying treasury contract..."
TREASURY_ID=$(stellar contract deploy \
    --network "$NETWORK" \
    --source-account default \
    "$TREASURY_WASM" \
    | grep "Contract ID" | awk '{print $NF}')

echo "✓ Treasury contract deployed: $TREASURY_ID"

echo "Initializing treasury with governance contract as admin..."
stellar contract invoke \
    --network "$NETWORK" \
    --source-account default \
    --id "$TREASURY_ID" \
    -- initialize \
    --admin "$GOVERNANCE_ID"

echo "✓ Treasury initialized - admin is governance contract: $GOVERNANCE_ID"
echo "  Disbursement flow: proposal -> governance vote -> multisig approval"

# Step 6: Transfer governance admin to multisig (Step 1 of two-step transfer)
echo "Initiating governance admin transfer to multisig..."
stellar contract invoke \
    --network "$NETWORK" \
    --source-account default \
    --id "$GOVERNANCE_ID" \
    -- transfer_admin \
    --admin "$TEMP_ADMIN" \
    --new_admin "$MULTISIG_ADDRESS"

echo "✓ Transfer initiated - governance pending_admin is now: $MULTISIG_ADDRESS"

# Step 7: Multisig accepts the governance admin role (Step 2 of two-step transfer)
echo ""
echo "⚠️  MANUAL STEP REQUIRED:"
echo "   Multisig signers must now execute the following to accept admin role:"
echo ""
echo "   stellar contract invoke \\"
echo "       --network $NETWORK \\"
echo "       --id $GOVERNANCE_ID \\"
echo "       -- accept_admin \\"
echo "       --pending_admin $MULTISIG_ADDRESS"
echo ""
echo "   After this transaction succeeds:"
echo "   - $MULTISIG_ADDRESS becomes the official governance admin"
echo "   - All future admin operations require multisig approval"
echo ""

# Step 8 (Optional): Transfer token admin to multisig
echo "To also transfer token admin to multisig, execute:"
echo ""
echo "   stellar contract invoke \\"
echo "       --network $NETWORK \\"
echo "       --id $TOKEN_ID \\"
echo "       -- transfer_admin \\"
echo "       --admin $TEMP_ADMIN \\"
echo "       --new_admin $MULTISIG_ADDRESS"
echo ""
echo "   Then multisig must accept with:"
echo "   stellar contract invoke \\"
echo "       --network $NETWORK \\"
echo "       --id $TOKEN_ID \\"
echo "       -- accept_admin \\"
echo "       --pending_admin $MULTISIG_ADDRESS"
echo ""

# Save deployment info
cat > "deployment_${NETWORK}_$(date +%s).json" << EOF
{
  "network": "$NETWORK",
  "token_id": "$TOKEN_ID",
  "governance_id": "$GOVERNANCE_ID",
  "treasury_id": "$TREASURY_ID",
  "treasury_admin": "$GOVERNANCE_ID",
  "temporary_admin": "$TEMP_ADMIN",
  "multisig_admin": "$MULTISIG_ADDRESS",
  "multisig_threshold": "$MULTISIG_THRESHOLD",
  "multisig_signers": ["$SIGNER_1", "$SIGNER_2", "$SIGNER_3"],
  "status": "pending_multisig_acceptance",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF

echo "✓ Deployment info saved"
