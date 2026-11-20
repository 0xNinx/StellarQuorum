#!/usr/bin/env bash
# Deploy Quorum contracts to Stellar testnet, mainnet, or a local network
# Usage: NETWORK=testnet SOURCE_ACCOUNT=<key alias or G...> bash scripts/deploy.sh
# Optional: DEPLOY_OUTPUT=path writes the deployed contract IDs to a JSON file.
set -euo pipefail

NETWORK="${NETWORK:-testnet}"
SOURCE_ACCOUNT="${SOURCE_ACCOUNT:?Set SOURCE_ACCOUNT to your Stellar account}"

case "$NETWORK" in
  testnet)
    RPC_URL="${RPC_URL:-https://soroban-testnet.stellar.org}"
    PASSPHRASE="${PASSPHRASE:-Test SDF Network ; September 2015}"
    ;;
  mainnet)
    RPC_URL="${RPC_URL:-https://soroban-rpc.stellar.org}"
    PASSPHRASE="${PASSPHRASE:-Public Global Stellar Network ; September 2015}"
    ;;
  local|standalone)
    RPC_URL="${RPC_URL:-http://localhost:8000/soroban/rpc}"
    PASSPHRASE="${PASSPHRASE:-Standalone Network ; February 2017}"
    ;;
  *)
    echo "ERROR: NETWORK must be local, standalone, testnet, or mainnet (got '$NETWORK')" >&2
    exit 1
    ;;
esac

INITIAL_SUPPLY="${INITIAL_SUPPLY:-100000000000000}"
if [[ -z "${GUARDIAN:-}" ]]; then
  GUARDIAN="$SOURCE_ACCOUNT"
  echo "WARNING: GUARDIAN not set; using deployer ${GUARDIAN} as guardian. Set GUARDIAN explicitly in production." >&2
fi
QUORUM_BPS="${QUORUM_BPS:-500}"
VOTING_PERIOD="${VOTING_PERIOD:-17280}"
TIMELOCK_PERIOD="${TIMELOCK_PERIOD:-34560}"
EXECUTION_GRACE_PERIOD="${EXECUTION_GRACE_PERIOD:-17280}"
PROPOSAL_THRESHOLD="${PROPOSAL_THRESHOLD:-500000000000}"

for value_name in INITIAL_SUPPLY QUORUM_BPS VOTING_PERIOD TIMELOCK_PERIOD EXECUTION_GRACE_PERIOD PROPOSAL_THRESHOLD; do
  value="${!value_name}"
  if [[ ! "$value" =~ ^[0-9]+$ ]]; then
    echo "ERROR: ${value_name} must be a non-negative integer (got '$value')" >&2
    exit 1
  fi
done

echo "Building contracts..."
(cd contracts && cargo build --target wasm32-unknown-unknown --release 2>&1)

WASM_DIR="contracts/target/wasm32-unknown-unknown/release"

echo "Deploying QUORUM token..."
TOKEN_ID=$(stellar contract deploy \
  --wasm "$WASM_DIR/quorum_token.wasm" \
  --source "$SOURCE_ACCOUNT" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$PASSPHRASE")
echo "  Token: $TOKEN_ID"

stellar contract invoke --id "$TOKEN_ID" --source "$SOURCE_ACCOUNT" \
  --rpc-url "$RPC_URL" --network-passphrase "$PASSPHRASE" \
  -- initialize --admin "$SOURCE_ACCOUNT" \
  --name "Quorum" --symbol "QUORUM" --decimals 7 \
  --initial_supply "$INITIAL_SUPPLY"

echo "Deploying Governance contract..."
GOV_ID=$(stellar contract deploy \
  --wasm "$WASM_DIR/quorum_governance.wasm" \
  --source "$SOURCE_ACCOUNT" \
  --rpc-url "$RPC_URL" \
  --network-passphrase "$PASSPHRASE")
echo "  Governance: $GOV_ID"

stellar contract invoke --id "$GOV_ID" --source "$SOURCE_ACCOUNT" \
  --rpc-url "$RPC_URL" --network-passphrase "$PASSPHRASE" \
  -- initialize --admin "$SOURCE_ACCOUNT" \
  --guardian "$GUARDIAN" \
  --token "$TOKEN_ID" --quorum_bps "$QUORUM_BPS" \
  --voting_period "$VOTING_PERIOD" --timelock_period "$TIMELOCK_PERIOD" \
  --execution_grace_period "$EXECUTION_GRACE_PERIOD" \
  --proposal_threshold "$PROPOSAL_THRESHOLD"

echo ""
echo "Done. Add to your .env.local:"
echo "NEXT_PUBLIC_GOVERNANCE_CONTRACT_ID=$GOV_ID"
echo "NEXT_PUBLIC_TOKEN_CONTRACT_ID=$TOKEN_ID"

# ---------------------------------------------------------------------------
# Upgrade an already-deployed governance contract
# ---------------------------------------------------------------------------
# To upgrade: build the new WASM, upload it, then submit an upgrade proposal.
#
# 1. Upload the new WASM and capture the hash:
#      NEW_HASH=$(stellar contract upload \
#        --wasm "$WASM_DIR/quorum_governance.wasm" \
#        --source "$SOURCE_ACCOUNT" \
#        --rpc-url "$RPC_URL" \
#        --network-passphrase "$PASSPHRASE")
#
# 2. Create a governance proposal with the action set to call `upgrade`:
#      stellar contract invoke --id "$GOV_ID" --source "$SOURCE_ACCOUNT" \
#        --rpc-url "$RPC_URL" --network-passphrase "$PASSPHRASE" \
#        -- set_proposal_action \
#        --proposer "$SOURCE_ACCOUNT" --proposal_id <ID> \
#        --target "$GOV_ID" --function upgrade
#
# 3. After the proposal passes and the timelock expires, execute it:
#      stellar contract invoke --id "$GOV_ID" --source "$SOURCE_ACCOUNT" \
#        --rpc-url "$RPC_URL" --network-passphrase "$PASSPHRASE" \
#        -- execute --executor "$SOURCE_ACCOUNT" --proposal_id <ID>
#
# The governance contract will call upgrade() on itself, replacing the WASM.
# Note: in production the admin should be the governance contract's own address
# so no single key can bypass the governance vote to trigger an upgrade.

if [[ -n "${DEPLOY_OUTPUT:-}" ]]; then
  cat > "$DEPLOY_OUTPUT" <<JSON
{
  "network": "$NETWORK",
  "commit": "$(git rev-parse HEAD 2>/dev/null || echo unknown)",
  "token_contract": "$TOKEN_ID",
  "governance_contract": "$GOV_ID"
}
JSON
  echo "Wrote deployment record to $DEPLOY_OUTPUT"
fi
