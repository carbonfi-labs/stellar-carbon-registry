# 🌍 Stellar Carbon Registry

A decentralized carbon credit tokenization, trading, and retirement protocol built on the Stellar network using Soroban smart contracts.

[![CI](https://github.com/carbonfi-labs/stellar-carbon-registry/actions/workflows/ci.yml/badge.svg)](https://github.com/carbonfi-labs/stellar-carbon-registry/actions/workflows/ci.yml)

## Overview

Stellar Carbon Registry lets an admin register carbon projects (reforestation, renewable energy, direct air capture), an independent **verifier** issue tokenized credits once real-world reduction is attested, project owners **list** credits for sale in USDC, and buyers **purchase** and permanently **retire** them. Every retirement mints an immutable, on-chain certificate.

## Why

Carbon markets suffer from double-counting, opaque registries, and slow settlement. Tokenizing credits on Stellar with Soroban gives us:
- **Transparent issuance** — every credit's origin is on-chain
- **Instant settlement** — USDC is transferred atomically with the credit purchase
- **Immutable retirement** — retired credits are burned and can never be re-traded
- **Low fees** — Stellar's sub-cent transactions make micro-credits viable

## Architecture

### Soroban contract (`contracts/carbon-registry`)

| Function | Auth | Description |
|---|---|---|
| `initialize(admin, verifier, usdc)` | — (one-shot) | Set admin, verifier, and the USDC token contract |
| `create_project(name, region, project_type, vintage) -> id` | admin | Register a new project |
| `issue_credits(project_id, amount)` | verifier | Mint credits to the project owner |
| `list_credits(project_id, price_per_credit, amount) -> listing_id` | owner | Escrow credits into a sale listing (priced in USDC) |
| `buy_credits(buyer, listing_id, amount)` | buyer | Transfer USDC buyer→seller and move credits to the buyer |
| `cancel_listing(listing_id)` | seller | Close a listing and return escrowed credits |
| `retire_credits(retiree, project_id, amount, reason) -> cert_id` | retiree | Burn credits and mint a retirement certificate |
| `get_project` / `get_listing` / `get_retirement` | view | Read state |
| `balance_of(holder, project_id)` | view | Credit balance a holder owns for a project |

Design highlights:
- **Typed errors** (`#[contracterror]`) instead of panics — the frontend gets stable numeric codes.
- **Overflow-checked arithmetic** (`checked_add` / `checked_mul`) on every credit and price accumulation.
- **Real USDC settlement** via the Stellar Asset Contract (SAC) token interface — works with canonical testnet/mainnet USDC.
- **Per-holder balances** with credits escrowed on listing and moved on purchase.
- **Events** on every state transition for off-chain indexing.
- **Storage TTL bumping** so instance and persistent entries don't expire.

### Frontend (`frontend`)

React + Vite + Freighter. Connects a wallet, browses projects, buys credits (signs a real `buy_credits` invocation), retires credits, and views retirement certificates. Reads are done via Soroban RPC simulation; writes are simulated, signed with Freighter, and submitted. The contract id is configured through `VITE_REGISTRY_CONTRACT_ID` (see `frontend/.env.example`).

## Lifecycle

1. **Admin** `create_project(...)` → project registered
2. **Verifier** `issue_credits(project_id, amount)` → credits minted to the owner
3. **Owner** `list_credits(project_id, price, amount)` → credits escrowed, listing opened
4. **Buyer** `buy_credits(listing_id, amount)` → USDC paid to seller, credits moved to buyer
5. **Buyer** `retire_credits(project_id, amount, reason)` → credits burned, certificate minted

## Build & test

Contract:
```bash
cd contracts/carbon-registry
# soroban-sdk 22 testutils currently need ed25519-dalek pinned to 2.x:
cargo update ed25519-dalek@3.0.0 --precise 2.1.1 || true
cargo test
cargo build --release --target wasm32v1-none
```

Frontend:
```bash
cd frontend
npm install
cp .env.example .env      # then set VITE_REGISTRY_CONTRACT_ID
npm run dev
```

## Deploy

See [docs/DEPLOY.md](docs/DEPLOY.md) for deploying the contract to testnet with the Stellar CLI and wiring the frontend to it.

## License

MIT
