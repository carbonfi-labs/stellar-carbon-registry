#![no_std]
//! # Stellar Carbon Registry
//!
//! A decentralized carbon-credit tokenization, trading, and retirement
//! protocol built on Soroban.
//!
//! ## Roles
//! - **Admin** — bootstraps the registry and creates projects.
//! - **Verifier** — an independent attester who mints (issues) credits for a
//!   project once real-world carbon reduction has been verified.
//! - **Project owner** — lists issued credits for sale (denominated in USDC).
//! - **Buyer** — purchases listed credits (USDC is transferred to the seller)
//!   and can later permanently retire them, generating an immutable
//!   retirement certificate.
//!
//! ## Design notes
//! - All arithmetic on credit amounts is overflow-checked (`checked_*`) and
//!   returns [`Error::Overflow`] instead of trapping.
//! - Expected failures return a typed [`Error`] rather than `panic!`, so the
//!   frontend gets stable numeric codes.
//! - USDC settlement uses the standard Stellar Asset Contract (SAC) token
//!   interface via [`token::Client`], so the registry works with the canonical
//!   testnet/mainnet USDC contract.
//! - Every state transition emits an event for off-chain indexing.

mod error;
mod events;
mod types;

#[cfg(test)]
mod test;

pub use error::Error;
pub use types::{DataKey, Listing, Project, Retirement};

use soroban_sdk::{contract, contractimpl, token, Address, Env, String};

/// Maximum length (in bytes) accepted for a metadata string. Keeps per-entry
/// storage bounded and rent predictable.
const MAX_STR_LEN: u32 = 128;

/// Ledgers to keep instance storage alive, and the threshold at which we bump.
/// ~ 30 days assuming 5s ledgers: 30 * 24 * 60 * 60 / 5 = 518_400.
const INSTANCE_TTL: u32 = 518_400;
const INSTANCE_TTL_THRESHOLD: u32 = INSTANCE_TTL - 17_280; // bump when < ~1 day left

/// Ledgers to keep persistent entries alive (~ 90 days).
const PERSISTENT_TTL: u32 = 1_555_200;
const PERSISTENT_TTL_THRESHOLD: u32 = PERSISTENT_TTL - 17_280;

#[contract]
pub struct CarbonRegistry;

#[contractimpl]
impl CarbonRegistry {
    /// One-time bootstrap. Sets the admin, verifier and USDC token contract.
    pub fn initialize(
        env: Env,
        admin: Address,
        verifier: Address,
        usdc: Address,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        let s = env.storage().instance();
        s.set(&DataKey::Admin, &admin);
        s.set(&DataKey::Verifier, &verifier);
        s.set(&DataKey::UsdcToken, &usdc);
        s.set(&DataKey::NextProjectId, &1u64);
        s.set(&DataKey::NextListingId, &1u64);
        s.set(&DataKey::NextRetirementId, &1u64);
        s.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL);

        events::initialized(&env, &admin, &verifier, &usdc);
        Ok(())
    }

    /// Admin registers a new project. Credits are minted later by the verifier.
    pub fn create_project(
        env: Env,
        name: String,
        region: String,
        project_type: String,
        vintage: u32,
    ) -> Result<u64, Error> {
        let admin = Self::admin(&env)?;
        admin.require_auth();

        if name.len() > MAX_STR_LEN
            || region.len() > MAX_STR_LEN
            || project_type.len() > MAX_STR_LEN
        {
            return Err(Error::MetadataTooLong);
        }

        let id = Self::bump_counter(&env, &DataKey::NextProjectId)?;
        let project = Project {
            owner: admin.clone(),
            name,
            region,
            project_type,
            vintage,
            total_issued: 0,
            available: 0,
            retired: 0,
        };
        Self::put_project(&env, id, &project);

        events::project_created(&env, id, &admin, project.vintage);
        Ok(id)
    }

    /// Verifier mints `amount` credits for an existing project. The newly
    /// issued credits are credited to the project owner's balance and to the
    /// project's `available` pool.
    pub fn issue_credits(env: Env, project_id: u64, amount: u128) -> Result<(), Error> {
        let verifier = Self::verifier(&env)?;
        verifier.require_auth();

        if amount == 0 {
            return Err(Error::InvalidAmount);
        }

        let mut project = Self::get_project_inner(&env, project_id)?;
        project.total_issued = project
            .total_issued
            .checked_add(amount)
            .ok_or(Error::Overflow)?;
        project.available = project
            .available
            .checked_add(amount)
            .ok_or(Error::Overflow)?;
        Self::put_project(&env, project_id, &project);

        Self::credit_balance(&env, &project.owner, project_id, amount)?;

        events::credits_issued(&env, project_id, amount, project.total_issued);
        Ok(())
    }

    /// Project owner lists `amount` of their available credits for sale at
    /// `price_per_credit` (USDC smallest unit). Returns the new listing id.
    pub fn list_credits(
        env: Env,
        project_id: u64,
        price_per_credit: u128,
        amount: u128,
    ) -> Result<u64, Error> {
        if amount == 0 {
            return Err(Error::InvalidAmount);
        }

        let mut project = Self::get_project_inner(&env, project_id)?;
        let seller = project.owner.clone();
        seller.require_auth();

        if project.available < amount {
            return Err(Error::InsufficientAvailable);
        }
        // Move credits out of the owner's spendable balance into escrow (the listing).
        Self::debit_balance(&env, &seller, project_id, amount)?;
        project.available -= amount; // safe: checked by InsufficientAvailable above
        Self::put_project(&env, project_id, &project);

        let id = Self::bump_counter(&env, &DataKey::NextListingId)?;
        let listing = Listing {
            project_id,
            seller: seller.clone(),
            price_per_credit,
            remaining: amount,
            active: true,
        };
        Self::put_listing(&env, id, &listing);

        events::listed(&env, id, project_id, price_per_credit, amount);
        Ok(id)
    }

    /// Buyer purchases `amount` credits from an active listing. USDC is
    /// transferred buyer -> seller for `amount * price_per_credit`, and the
    /// credits are moved into the buyer's balance.
    pub fn buy_credits(
        env: Env,
        buyer: Address,
        listing_id: u64,
        amount: u128,
    ) -> Result<(), Error> {
        buyer.require_auth();
        if amount == 0 {
            return Err(Error::InvalidAmount);
        }

        let mut listing = Self::get_listing_inner(&env, listing_id)?;
        if !listing.active || listing.remaining == 0 {
            return Err(Error::ListingNotFound);
        }
        if listing.remaining < amount {
            return Err(Error::InsufficientListed);
        }

        let total_price = amount
            .checked_mul(listing.price_per_credit)
            .ok_or(Error::Overflow)?;

        // Settle USDC buyer -> seller. i128 is the SAC amount type; guard the cast.
        if total_price > i128::MAX as u128 {
            return Err(Error::Overflow);
        }
        let usdc = Self::usdc(&env)?;
        let token_client = token::Client::new(&env, &usdc);
        token_client.transfer(&buyer, &listing.seller, &(total_price as i128));

        // Move credits from escrow to the buyer.
        listing.remaining -= amount; // safe: checked by InsufficientListed above
        if listing.remaining == 0 {
            listing.active = false;
        }
        Self::put_listing(&env, listing_id, &listing);
        Self::credit_balance(&env, &buyer, listing.project_id, amount)?;

        events::purchased(&env, listing_id, &buyer, amount, total_price);
        Ok(())
    }

    /// Seller cancels a listing, returning any remaining escrowed credits to
    /// their available balance.
    pub fn cancel_listing(env: Env, listing_id: u64) -> Result<(), Error> {
        let mut listing = Self::get_listing_inner(&env, listing_id)?;
        listing.seller.require_auth();
        if !listing.active {
            return Err(Error::ListingNotFound);
        }

        let remaining = listing.remaining;
        listing.remaining = 0;
        listing.active = false;
        Self::put_listing(&env, listing_id, &listing);

        if remaining > 0 {
            let mut project = Self::get_project_inner(&env, listing.project_id)?;
            project.available = project
                .available
                .checked_add(remaining)
                .ok_or(Error::Overflow)?;
            Self::put_project(&env, listing.project_id, &project);
            Self::credit_balance(&env, &listing.seller, listing.project_id, remaining)?;
        }

        events::listing_cancelled(&env, listing_id, remaining);
        Ok(())
    }

    /// Permanently retire `amount` of the caller's credits for a project.
    /// Burns the credits (they can never be re-traded) and mints an immutable
    /// retirement certificate. Returns the certificate id.
    pub fn retire_credits(
        env: Env,
        retiree: Address,
        project_id: u64,
        amount: u128,
        reason: String,
    ) -> Result<u64, Error> {
        retiree.require_auth();
        if amount == 0 {
            return Err(Error::InvalidAmount);
        }
        if reason.len() > MAX_STR_LEN {
            return Err(Error::MetadataTooLong);
        }

        let mut project = Self::get_project_inner(&env, project_id)?;

        // Burn from the retiree's balance.
        Self::debit_balance(&env, &retiree, project_id, amount)?;
        project.retired = project.retired.checked_add(amount).ok_or(Error::Overflow)?;
        Self::put_project(&env, project_id, &project);

        let id = Self::bump_counter(&env, &DataKey::NextRetirementId)?;
        let retirement = Retirement {
            retiree: retiree.clone(),
            project_id,
            amount,
            reason,
            timestamp: env.ledger().timestamp(),
        };
        Self::put_retirement(&env, id, &retirement);

        events::retired(&env, id, project_id, &retiree, amount);
        Ok(id)
    }

    // ----------------------------- views ---------------------------------

    pub fn get_project(env: Env, project_id: u64) -> Result<Project, Error> {
        Self::get_project_inner(&env, project_id)
    }

    pub fn get_listing(env: Env, listing_id: u64) -> Result<Listing, Error> {
        Self::get_listing_inner(&env, listing_id)
    }

    pub fn get_retirement(env: Env, retirement_id: u64) -> Result<Retirement, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Retirement(retirement_id))
            .ok_or(Error::RetirementNotFound)
    }

    /// Credit balance a holder owns for a specific project.
    pub fn balance_of(env: Env, holder: Address, project_id: u64) -> u128 {
        env.storage()
            .persistent()
            .get(&DataKey::Balance(holder, project_id))
            .unwrap_or(0)
    }

    pub fn admin_address(env: Env) -> Result<Address, Error> {
        Self::admin(&env)
    }

    pub fn verifier_address(env: Env) -> Result<Address, Error> {
        Self::verifier(&env)
    }

    pub fn usdc_address(env: Env) -> Result<Address, Error> {
        Self::usdc(&env)
    }

    // --------------------------- internals --------------------------------

    fn admin(env: &Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)
    }

    fn verifier(env: &Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Verifier)
            .ok_or(Error::NotInitialized)
    }

    fn usdc(env: &Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::UsdcToken)
            .ok_or(Error::NotInitialized)
    }

    /// Read the counter at `key`, return its current value, and persist value+1.
    fn bump_counter(env: &Env, key: &DataKey) -> Result<u64, Error> {
        let s = env.storage().instance();
        let id: u64 = s.get(key).ok_or(Error::NotInitialized)?;
        s.set(key, &id.checked_add(1).ok_or(Error::Overflow)?);
        s.extend_ttl(INSTANCE_TTL_THRESHOLD, INSTANCE_TTL);
        Ok(id)
    }

    fn get_project_inner(env: &Env, id: u64) -> Result<Project, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Project(id))
            .ok_or(Error::ProjectNotFound)
    }

    fn get_listing_inner(env: &Env, id: u64) -> Result<Listing, Error> {
        env.storage()
            .persistent()
            .get(&DataKey::Listing(id))
            .ok_or(Error::ListingNotFound)
    }

    fn put_project(env: &Env, id: u64, project: &Project) {
        let key = DataKey::Project(id);
        env.storage().persistent().set(&key, project);
        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL);
    }

    fn put_listing(env: &Env, id: u64, listing: &Listing) {
        let key = DataKey::Listing(id);
        env.storage().persistent().set(&key, listing);
        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL);
    }

    fn put_retirement(env: &Env, id: u64, retirement: &Retirement) {
        let key = DataKey::Retirement(id);
        env.storage().persistent().set(&key, retirement);
        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL);
    }

    /// Add `amount` to a holder's per-project balance (overflow-checked).
    fn credit_balance(
        env: &Env,
        holder: &Address,
        project_id: u64,
        amount: u128,
    ) -> Result<(), Error> {
        let key = DataKey::Balance(holder.clone(), project_id);
        let cur: u128 = env.storage().persistent().get(&key).unwrap_or(0);
        let next = cur.checked_add(amount).ok_or(Error::Overflow)?;
        env.storage().persistent().set(&key, &next);
        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL);
        Ok(())
    }

    /// Subtract `amount` from a holder's per-project balance, erroring if the
    /// holder does not have enough.
    fn debit_balance(
        env: &Env,
        holder: &Address,
        project_id: u64,
        amount: u128,
    ) -> Result<(), Error> {
        let key = DataKey::Balance(holder.clone(), project_id);
        let cur: u128 = env.storage().persistent().get(&key).unwrap_or(0);
        if cur < amount {
            return Err(Error::InsufficientBalance);
        }
        env.storage().persistent().set(&key, &(cur - amount));
        env.storage()
            .persistent()
            .extend_ttl(&key, PERSISTENT_TTL_THRESHOLD, PERSISTENT_TTL);
        Ok(())
    }
}
