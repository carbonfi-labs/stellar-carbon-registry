# Changelog

All notable changes to this project are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.2.0] — Protocol rewrite

This release turns the initial skeleton into a working, tested protocol. The
original contract did not compile against any current soroban-sdk and advertised
functionality (USDC purchases, retirement certificates) that was not implemented.

### Added
- **Real `buy_credits`** with USDC settlement via the Stellar Asset Contract
  (SAC) token interface — USDC is transferred buyer→seller atomically with the
  credit transfer.
- **Per-holder credit balances** (`balance_of`) and a `Balance(Address, project_id)`
  storage model. Credits are escrowed on `list_credits` and moved to the buyer on
  purchase.
- **`cancel_listing`** to close a listing and return escrowed credits to the owner.
- **Typed error enum** (`#[contracterror]`) covering all expected failures
  (`ProjectNotFound`, `InsufficientListed`, `Overflow`, auth errors, …) instead
  of `panic!`.
- **Events** on every state transition (`init`, `project`, `issue`, `list`,
  `buy`, `cancel`, `retire`) for off-chain indexing.
- **Storage TTL bumping** for instance and persistent entries.
- **Retirement certificates** tracked with a `retired` tally per project.
- **Comprehensive test suite** (14 tests) using a mock USDC SAC: full
  issue→list→buy→retire lifecycle plus failure/overflow/auth/bounds cases.
- **GitHub Actions CI**: `cargo fmt --check`, `clippy -D warnings`, `cargo test`,
  wasm build, and frontend typecheck/build.
- **Frontend contract integration**: a typed registry client
  (`frontend/src/registry.ts`) and UI panels to browse projects, buy, retire, and
  view certificates — replacing the generic "send XLM" demo. Network/contract
  configuration via `VITE_*` env (`frontend/.env.example`).
- **Deploy guide** (`docs/DEPLOY.md`).

### Changed
- Upgraded `soroban-sdk` from 20.0.0 to 22.
- Migrated off the removed storage API (`env.storage().set/get`) to the
  `instance()` / `persistent()` split, and off `env.invoker()` to explicit
  `Address` + `require_auth()`.
- `list_credits` is now authorized by the project owner (not a hardcoded admin
  read) and escrows credits.
- README rewritten to match the actual contract API.

### Fixed
- **Overflow checks in balance accumulation** (issues #14 / #27) — all credit and
  price math uses `checked_add` / `checked_mul` and returns `Error::Overflow`.
- Retirement now correctly decrements the retiree's balance and records the
  burn against the project (the original never tracked holder balances, so
  buying/retiring could not work end-to-end).
- Contract now compiles and its wasm builds (the 0.1 skeleton did neither).

## [0.1.0] — Initial skeleton
- Initial project scaffold: contract data types, a partial `frontend` Freighter
  wallet demo, and README describing the intended protocol.
