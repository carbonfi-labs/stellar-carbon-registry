use soroban_sdk::{contracttype, Address, String};

/// Storage keys. Instance storage holds config + counters; persistent storage
/// holds the (potentially unbounded) collections of projects, listings,
/// retirements and per-holder balances.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Config / singletons (instance storage).
    Admin,
    Verifier,
    UsdcToken,
    NextProjectId,
    NextListingId,
    NextRetirementId,

    /// Collections (persistent storage).
    Project(u64),
    Listing(u64),
    Retirement(u64),
    /// Credit balance a holder owns for a specific project.
    Balance(Address, u64),
}

/// A carbon-credit-issuing project (reforestation, renewable energy, DAC, …).
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    pub owner: Address,
    pub name: String,
    pub region: String,
    pub project_type: String,
    pub vintage: u32,
    /// Total credits ever minted for this project.
    pub total_issued: u128,
    /// Credits held by the project owner that are not currently listed.
    pub available: u128,
    /// Cumulative credits retired against this project.
    pub retired: u128,
}

/// An open sale offer of credits at a fixed USDC price.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Listing {
    pub project_id: u64,
    pub seller: Address,
    /// Price of a single credit, denominated in the USDC token's smallest unit.
    pub price_per_credit: u128,
    /// Credits still available under this listing.
    pub remaining: u128,
    pub active: bool,
}

/// A permanent, immutable retirement certificate.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Retirement {
    pub retiree: Address,
    pub project_id: u64,
    pub amount: u128,
    pub reason: String,
    pub timestamp: u64,
}
