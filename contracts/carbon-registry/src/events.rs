//! Event emission helpers. Each state transition publishes a typed topic so
//! off-chain indexers (and the frontend) can follow the registry without
//! polling every storage entry.

use soroban_sdk::{symbol_short, Address, Env};

pub(crate) fn initialized(env: &Env, admin: &Address, verifier: &Address, usdc: &Address) {
    env.events().publish(
        (symbol_short!("init"),),
        (admin.clone(), verifier.clone(), usdc.clone()),
    );
}

pub(crate) fn project_created(env: &Env, project_id: u64, owner: &Address, vintage: u32) {
    env.events().publish(
        (symbol_short!("project"), project_id),
        (owner.clone(), vintage),
    );
}

pub(crate) fn credits_issued(env: &Env, project_id: u64, amount: u128, total_issued: u128) {
    env.events()
        .publish((symbol_short!("issue"), project_id), (amount, total_issued));
}

pub(crate) fn listed(
    env: &Env,
    listing_id: u64,
    project_id: u64,
    price_per_credit: u128,
    amount: u128,
) {
    env.events().publish(
        (symbol_short!("list"), listing_id),
        (project_id, price_per_credit, amount),
    );
}

pub(crate) fn purchased(
    env: &Env,
    listing_id: u64,
    buyer: &Address,
    amount: u128,
    total_price: u128,
) {
    env.events().publish(
        (symbol_short!("buy"), listing_id),
        (buyer.clone(), amount, total_price),
    );
}

pub(crate) fn listing_cancelled(env: &Env, listing_id: u64, returned: u128) {
    env.events()
        .publish((symbol_short!("cancel"), listing_id), returned);
}

pub(crate) fn retired(
    env: &Env,
    retirement_id: u64,
    project_id: u64,
    retiree: &Address,
    amount: u128,
) {
    env.events().publish(
        (symbol_short!("retire"), retirement_id),
        (project_id, retiree.clone(), amount),
    );
}
