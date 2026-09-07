# Carbon Registry — Frontend

React + Vite + TypeScript web app for the [Stellar Carbon Registry](../README.md)
Soroban contract. It connects a **Freighter** wallet and lets a user browse
projects, buy listed credits (paying USDC), permanently retire credits, and view
retirement certificates.

Built with [`@stellar/freighter-api`](https://developers.stellar.org/docs/build/freighter)
for wallet integration and [`stellar-sdk`](https://developers.stellar.org/docs/data-and-tools/stellar-sdk)
for Soroban RPC (contract simulation + submission) and Horizon (XLM balance).

## How it talks to the contract

- **Reads** (`get_project`, `get_listing`, `get_retirement`, `balance_of`) are run
  as Soroban RPC **simulations** — no signing, no fee.
- **Writes** (`buy_credits`, `retire_credits`, …) are assembled, simulated to
  compute the footprint, **signed with Freighter**, submitted, and polled to
  confirmation. See `src/registry.ts`.

## Configuration

Copy `.env.example` to `.env` and set your deployed contract id:

```bash
cp .env.example .env
# VITE_REGISTRY_CONTRACT_ID=C...   (from `stellar contract deploy`)
```

| Variable | Default | Purpose |
|---|---|---|
| `VITE_NETWORK` | `TESTNET` | `TESTNET` or `PUBLIC` |
| `VITE_REGISTRY_CONTRACT_ID` | — | The deployed registry contract id |
| `VITE_HORIZON_URL` | testnet Horizon | Horizon endpoint |
| `VITE_SOROBAN_RPC_URL` | testnet RPC | Soroban RPC endpoint |

See [../docs/DEPLOY.md](../docs/DEPLOY.md) to deploy the contract first.

## Setup

### Prerequisites
- Node.js 18+ and npm
- The [Freighter](https://www.freighter.app/) browser extension, set to **Testnet**

### Install & run
```bash
npm install
npm run dev      # Vite dev server at http://localhost:5173
```

### Build
```bash
npm run build    # tsc typecheck + production build -> dist/
npm run preview
```

## Project structure

```
src
├── main.tsx        # React entry
├── App.tsx         # UI: wallet, project explorer, buy, retire, certificate viewer
├── wallet.ts       # Freighter integration (connect/disconnect/sign)
├── registry.ts     # Carbon Registry contract client (reads via sim, writes via Freighter)
├── stellar.ts      # Horizon helpers (XLM balance) + network re-exports
├── config.ts       # Network + contract-id configuration from VITE_* env
└── styles.css
```
