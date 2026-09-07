#!/usr/bin/env bash
#
# One-shot deploy helper for the Stellar Carbon Registry contract.
#
# Builds the wasm, deploys it to the chosen network, initializes it with an
# admin/verifier/USDC token, and (optionally) writes the resulting contract id
# into frontend/.env as VITE_REGISTRY_CONTRACT_ID.
#
# Prerequisites:
#   - Rust with the wasm target:  rustup target add wasm32v1-none
#   - Stellar CLI:                cargo install --locked stellar-cli
#   - A funded identity:          stellar keys generate --global admin --network testnet --fund
#
# Usage:
#   scripts/deploy.sh [--network testnet|mainnet] \
#                     [--source admin] \
#                     [--verifier <G...|identity>] \
#                     [--usdc <USDC_CONTRACT_ID>] \
#                     [--no-init] [--write-env]
#
# Examples:
#   # Deploy + initialize on testnet, verifier defaults to the admin identity,
#   # and write the contract id into frontend/.env:
#   scripts/deploy.sh --usdc CBIELTK6YBZJU5UP2WWQEUCYKLPU6AUNZ2BQ4WWFEIE3USCIHMXQDAMA --write-env
#
#   # Just build + deploy, skip initialize:
#   scripts/deploy.sh --no-init
set -euo pipefail

# --- defaults -------------------------------------------------------------
NETWORK="testnet"
SOURCE="admin"
VERIFIER=""          # defaults to $SOURCE's address if unset
USDC=""              # required unless --no-init
DO_INIT=1
WRITE_ENV=0

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONTRACT_DIR="$REPO_ROOT/contracts/carbon-registry"
WASM="$CONTRACT_DIR/target/wasm32v1-none/release/carbon_registry.wasm"
ENV_FILE="$REPO_ROOT/frontend/.env"

# --- args -----------------------------------------------------------------
while [[ $# -gt 0 ]]; do
  case "$1" in
    --network)  NETWORK="$2"; shift 2 ;;
    --source)   SOURCE="$2"; shift 2 ;;
    --verifier) VERIFIER="$2"; shift 2 ;;
    --usdc)     USDC="$2"; shift 2 ;;
    --no-init)  DO_INIT=0; shift ;;
    --write-env) WRITE_ENV=1; shift ;;
    -h|--help)  grep '^#' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "Unknown argument: $1" >&2; exit 1 ;;
  esac
done

command -v stellar >/dev/null 2>&1 || {
  echo "error: the 'stellar' CLI is not installed. Run: cargo install --locked stellar-cli" >&2
  exit 1
}

# --- build ----------------------------------------------------------------
echo "==> Building contract (release, wasm32v1-none)"
pushd "$CONTRACT_DIR" >/dev/null
# soroban-sdk 22 testutils pull ed25519-dalek 3.x transitively, which currently
# fails to compile; pin to the 2.x line. Harmless once upstream is fixed.
cargo update ed25519-dalek@3.0.0 --precise 2.1.1 >/dev/null 2>&1 || true
cargo build --release --target wasm32v1-none
popd >/dev/null

[[ -f "$WASM" ]] || { echo "error: wasm artifact not found at $WASM" >&2; exit 1; }
echo "    wasm: $WASM ($(wc -c < "$WASM") bytes)"

# --- deploy ---------------------------------------------------------------
echo "==> Deploying to $NETWORK (source: $SOURCE)"
CONTRACT_ID="$(stellar contract deploy \
  --wasm "$WASM" \
  --source "$SOURCE" \
  --network "$NETWORK")"
echo "    contract id: $CONTRACT_ID"

# --- initialize -----------------------------------------------------------
if [[ "$DO_INIT" -eq 1 ]]; then
  [[ -n "$USDC" ]] || {
    echo "error: --usdc <USDC_CONTRACT_ID> is required to initialize (or pass --no-init)." >&2
    exit 1
  }
  ADMIN_ADDR="$(stellar keys address "$SOURCE")"
  if [[ -z "$VERIFIER" ]]; then
    VERIFIER="$ADMIN_ADDR"
  elif [[ "$VERIFIER" != G* ]]; then
    # Treat a non-G value as an identity name and resolve it.
    VERIFIER="$(stellar keys address "$VERIFIER")"
  fi

  echo "==> Initializing (admin=$ADMIN_ADDR, verifier=$VERIFIER, usdc=$USDC)"
  stellar contract invoke \
    --id "$CONTRACT_ID" --source "$SOURCE" --network "$NETWORK" \
    -- initialize \
    --admin "$ADMIN_ADDR" \
    --verifier "$VERIFIER" \
    --usdc "$USDC"
  echo "    initialized."
fi

# --- write env ------------------------------------------------------------
if [[ "$WRITE_ENV" -eq 1 ]]; then
  echo "==> Writing VITE_REGISTRY_CONTRACT_ID to $ENV_FILE"
  touch "$ENV_FILE"
  # Replace existing line or append.
  if grep -q '^VITE_REGISTRY_CONTRACT_ID=' "$ENV_FILE"; then
    sed -i.bak "s|^VITE_REGISTRY_CONTRACT_ID=.*|VITE_REGISTRY_CONTRACT_ID=$CONTRACT_ID|" "$ENV_FILE"
    rm -f "$ENV_FILE.bak"
  else
    printf 'VITE_REGISTRY_CONTRACT_ID=%s\n' "$CONTRACT_ID" >> "$ENV_FILE"
  fi
fi

echo
echo "Done. Contract id: $CONTRACT_ID"
echo "Explore: https://stellar.expert/explorer/${NETWORK}/contract/${CONTRACT_ID}"
