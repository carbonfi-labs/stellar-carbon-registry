#![cfg(test)]

use crate::{CarbonRegistry, CarbonRegistryClient, Error};
use soroban_sdk::{
    testutils::Address as _,
    token::{StellarAssetClient, TokenClient},
    Address, Env, String,
};

/// Deploy a fresh registry plus a mock USDC Stellar Asset Contract.
/// Returns (env, registry client, usdc admin client, usdc token client,
/// admin, verifier).
fn setup<'a>() -> (
    Env,
    CarbonRegistryClient<'a>,
    StellarAssetClient<'a>,
    TokenClient<'a>,
    Address,
    Address,
) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let verifier = Address::generate(&env);

    // Mock USDC as a Stellar Asset Contract.
    let issuer = Address::generate(&env);
    let sac = env.register_stellar_asset_contract_v2(issuer);
    let usdc_id = sac.address();
    let usdc_admin = StellarAssetClient::new(&env, &usdc_id);
    let usdc_token = TokenClient::new(&env, &usdc_id);

    let contract_id = env.register(CarbonRegistry, ());
    let registry = CarbonRegistryClient::new(&env, &contract_id);
    registry.initialize(&admin, &verifier, &usdc_id);

    (env, registry, usdc_admin, usdc_token, admin, verifier)
}

fn s(env: &Env, v: &str) -> String {
    String::from_str(env, v)
}

#[test]
fn initialize_sets_config() {
    let (_env, registry, _usdc_admin, _usdc, admin, verifier) = setup();
    assert_eq!(registry.admin_address(), admin);
    assert_eq!(registry.verifier_address(), verifier);
}

#[test]
fn initialize_is_one_shot() {
    let (_env, registry, _ua, _u, admin, verifier) = setup();
    let other = registry.usdc_address();
    let res = registry.try_initialize(&admin, &verifier, &other);
    assert_eq!(res, Err(Ok(Error::AlreadyInitialized)));
}

#[test]
fn create_and_issue_flow() {
    let (env, registry, _ua, _u, admin, _verifier) = setup();

    let pid = registry.create_project(
        &s(&env, "Amazon Reforestation"),
        &s(&env, "BR"),
        &s(&env, "reforestation"),
        &2025,
    );
    assert_eq!(pid, 1);

    registry.issue_credits(&pid, &1_000);
    let project = registry.get_project(&pid);
    assert_eq!(project.total_issued, 1_000);
    assert_eq!(project.available, 1_000);
    // Issued credits are credited to the project owner (admin).
    assert_eq!(registry.balance_of(&admin, &pid), 1_000);
}

#[test]
fn issue_zero_is_rejected() {
    let (env, registry, _ua, _u, _admin, _verifier) = setup();
    let pid = registry.create_project(&s(&env, "P"), &s(&env, "R"), &s(&env, "t"), &2025);
    assert_eq!(
        registry.try_issue_credits(&pid, &0),
        Err(Ok(Error::InvalidAmount))
    );
}

#[test]
fn issue_unknown_project_errors() {
    let (_env, registry, _ua, _u, _admin, _verifier) = setup();
    assert_eq!(
        registry.try_issue_credits(&999, &10),
        Err(Ok(Error::ProjectNotFound))
    );
}

#[test]
fn full_buy_and_retire_lifecycle() {
    let (env, registry, usdc_admin, usdc, admin, _verifier) = setup();

    // Project + issuance.
    let pid = registry.create_project(
        &s(&env, "Solar Farm"),
        &s(&env, "KE"),
        &s(&env, "renewable"),
        &2025,
    );
    registry.issue_credits(&pid, &500);

    // Owner lists 300 credits at 2 USDC each.
    let listing_id = registry.list_credits(&pid, &2, &300);
    let project = registry.get_project(&pid);
    assert_eq!(project.available, 200); // 500 - 300 escrowed
    assert_eq!(registry.balance_of(&admin, &pid), 200);

    // Fund a buyer with USDC and purchase 100 credits.
    let buyer = Address::generate(&env);
    usdc_admin.mint(&buyer, &1_000);
    registry.buy_credits(&buyer, &listing_id, &100);

    // USDC moved buyer -> seller: 100 * 2 = 200.
    assert_eq!(usdc.balance(&buyer), 800);
    assert_eq!(usdc.balance(&admin), 200);
    // Buyer now holds 100 credits; listing has 200 remaining.
    assert_eq!(registry.balance_of(&buyer, &pid), 100);
    assert_eq!(registry.get_listing(&listing_id).remaining, 200);

    // Buyer retires 40 credits.
    let cert_id = registry.retire_credits(&buyer, &pid, &40, &s(&env, "offset Q3 travel"));
    assert_eq!(cert_id, 1);
    assert_eq!(registry.balance_of(&buyer, &pid), 60);
    let cert = registry.get_retirement(&cert_id);
    assert_eq!(cert.retiree, buyer);
    assert_eq!(cert.amount, 40);
    assert_eq!(registry.get_project(&pid).retired, 40);
}

#[test]
fn buy_more_than_listed_errors() {
    let (env, registry, usdc_admin, _u, _admin, _verifier) = setup();
    let pid = registry.create_project(&s(&env, "P"), &s(&env, "R"), &s(&env, "t"), &2025);
    registry.issue_credits(&pid, &100);
    let lid = registry.list_credits(&pid, &1, &50);

    let buyer = Address::generate(&env);
    usdc_admin.mint(&buyer, &1_000);
    assert_eq!(
        registry.try_buy_credits(&buyer, &lid, &51),
        Err(Ok(Error::InsufficientListed))
    );
}

#[test]
fn list_more_than_available_errors() {
    let (env, registry, _ua, _u, _admin, _verifier) = setup();
    let pid = registry.create_project(&s(&env, "P"), &s(&env, "R"), &s(&env, "t"), &2025);
    registry.issue_credits(&pid, &100);
    assert_eq!(
        registry.try_list_credits(&pid, &1, &101),
        Err(Ok(Error::InsufficientAvailable))
    );
}

#[test]
fn retire_more_than_balance_errors() {
    let (env, registry, _ua, _u, admin, _verifier) = setup();
    let pid = registry.create_project(&s(&env, "P"), &s(&env, "R"), &s(&env, "t"), &2025);
    registry.issue_credits(&pid, &10);
    assert_eq!(
        registry.try_retire_credits(&admin, &pid, &11, &s(&env, "x")),
        Err(Ok(Error::InsufficientBalance))
    );
}

#[test]
fn cancel_listing_returns_credits() {
    let (env, registry, _ua, _u, admin, _verifier) = setup();
    let pid = registry.create_project(&s(&env, "P"), &s(&env, "R"), &s(&env, "t"), &2025);
    registry.issue_credits(&pid, &100);
    let lid = registry.list_credits(&pid, &5, &60);
    assert_eq!(registry.balance_of(&admin, &pid), 40);

    registry.cancel_listing(&lid);
    // Escrowed credits returned to the owner.
    assert_eq!(registry.balance_of(&admin, &pid), 100);
    assert_eq!(registry.get_project(&pid).available, 100);
    assert!(!registry.get_listing(&lid).active);
}

#[test]
fn buy_from_cancelled_listing_errors() {
    let (env, registry, usdc_admin, _u, _admin, _verifier) = setup();
    let pid = registry.create_project(&s(&env, "P"), &s(&env, "R"), &s(&env, "t"), &2025);
    registry.issue_credits(&pid, &100);
    let lid = registry.list_credits(&pid, &1, &50);
    registry.cancel_listing(&lid);

    let buyer = Address::generate(&env);
    usdc_admin.mint(&buyer, &100);
    assert_eq!(
        registry.try_buy_credits(&buyer, &lid, &1),
        Err(Ok(Error::ListingNotFound))
    );
}

#[test]
fn selling_out_a_listing_deactivates_it() {
    let (env, registry, usdc_admin, usdc, admin, _verifier) = setup();
    let pid = registry.create_project(&s(&env, "P"), &s(&env, "R"), &s(&env, "t"), &2025);
    registry.issue_credits(&pid, &100);
    let lid = registry.list_credits(&pid, &3, &100);

    let buyer = Address::generate(&env);
    usdc_admin.mint(&buyer, &1_000);
    registry.buy_credits(&buyer, &lid, &100);

    let listing = registry.get_listing(&lid);
    assert_eq!(listing.remaining, 0);
    assert!(!listing.active);
    assert_eq!(usdc.balance(&admin), 300);
    assert_eq!(registry.balance_of(&buyer, &pid), 100);
}

#[test]
fn metadata_length_is_bounded() {
    let (env, registry, _ua, _u, _admin, _verifier) = setup();
    let long = String::from_str(
        &env,
        "this-metadata-string-is-deliberately-far-too-long-to-be-accepted-by-the-registry-contract-because-it-exceeds-the-configured-maximum-length-limit",
    );
    assert_eq!(
        registry.try_create_project(&long, &s(&env, "R"), &s(&env, "t"), &2025),
        Err(Ok(Error::MetadataTooLong))
    );
}

#[test]
fn views_error_on_missing_entities() {
    let (_env, registry, _ua, _u, _admin, _verifier) = setup();
    assert_eq!(
        registry.try_get_project(&42),
        Err(Ok(Error::ProjectNotFound))
    );
    assert_eq!(
        registry.try_get_listing(&42),
        Err(Ok(Error::ListingNotFound))
    );
    assert_eq!(
        registry.try_get_retirement(&42),
        Err(Ok(Error::RetirementNotFound))
    );
}
