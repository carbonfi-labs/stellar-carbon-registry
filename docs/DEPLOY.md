# Deploying to Testnet

This guide deploys the carbon registry contract to Stellar testnet and wires the
frontend to it.

> **TL;DR** — once you have the Stellar CLI and a funded identity, the whole
> flow is one command:
> ```bash
> scripts/deploy.sh --usdc <USDC_CONTRACT_ID> --write-env
> ```
> That builds the wasm, deploys, initializes, and writes
> `VITE_REGISTRY_CONTRACT_ID` into `frontend/.env`. The manual steps below
> explain what it does.

## Prerequisites

- Rust with the `wasm32v1-none` target: `rustup target add wasm32v1-none`
- The Stellar CLI: `cargo install --locked stellar-cli`
- A funded testnet identity:
  ```bash
  stellar keys generate --global admin --network testnet --fund
  stellar keys address admin
  ```

## 1. Build the contract

```bash
cd contracts/carbon-registry
cargo update ed25519-dalek@3.0.0 --precise 2.1.1 || true   # see README note
cargo build --release --target wasm32v1-none
```

The artifact is at
`target/wasm32v1-none/release/carbon_registry.wasm`.

## 2. Deploy

```bash
stellar contract deploy \
  --wasm target/wasm32v1-none/release/carbon_registry.wasm \
  --source admin \
  --network testnet
```

This prints the contract id (starts with `C…`). Save it.

## 3. Initialize

You need three addresses: the admin, the verifier, and the USDC token contract.
On testnet you can wrap a test USDC asset into a SAC, or deploy your own test
token. Then:

```bash
stellar contract invoke \
  --id <CONTRACT_ID> --source admin --network testnet \
  -- initialize \
  --admin <ADMIN_ADDRESS> \
  --verifier <VERIFIER_ADDRESS> \
  --usdc <USDC_CONTRACT_ID>
```

## 4. Try the lifecycle

```bash
# Admin creates a project
stellar contract invoke --id <CONTRACT_ID> --source admin --network testnet \
  -- create_project --name "Amazon Reforestation" --region "BR" --project_type "reforestation" --vintage 2025

# Verifier issues credits (use the verifier identity)
stellar contract invoke --id <CONTRACT_ID> --source verifier --network testnet \
  -- issue_credits --project_id 1 --amount 1000

# Owner lists 300 credits at 2 USDC each
stellar contract invoke --id <CONTRACT_ID> --source admin --network testnet \
  -- list_credits --project_id 1 --price_per_credit 2 --amount 300
```

## 5. Wire the frontend

```bash
cd frontend
cp .env.example .env
# set VITE_REGISTRY_CONTRACT_ID=<CONTRACT_ID>
npm install
npm run dev
```

Open the app, connect Freighter (set to Testnet), load project #1, and try
buying and retiring credits.
