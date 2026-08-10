#![allow(dead_code)]

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Map, String, Vec, Symbol};

// fix(#13): rounding error in accrued interest
#[contracttype]
pub enum DataKey {
    Admin,
    Verifier,
    UsdcToken,
    Project(u64),
    Listing(u64),
    Retirement(u64),
    NextProjectId,
    NextListingId,
    NextRetirementId,
    Balances(Address),
}

#[contracttype]
pub struct Project {
    pub owner: Address,
    pub name: String,
    pub region: String,
    pub project_type: String,
    pub vintage: u32,
    pub total_issued: u128,
    pub available: u128,
}

#[contracttype]
pub struct Listing {
    pub project_id: u64,
    pub seller: Address,
    pub price_per_credit: u128,
    pub amount: u128,
}

#[contracttype]
pub struct Retirement {
    pub retiree: Address,
    pub project_id: u64,
    pub amount: u128,
    pub reason: String,
    pub timestamp: u64,
}

#[contract]
pub struct CarbonRegistry;

#[contractimpl]
impl CarbonRegistry {
    pub fn initialize(env: Env, admin: Address, verifier: Address, usdc: Address) {
        env.storage().set(&DataKey::Admin, &admin);
        env.storage().set(&DataKey::Verifier, &verifier);
        env.storage().set(&DataKey::UsdcToken, &usdc);
        env.storage().set(&DataKey::NextProjectId, &1u64);
        env.storage().set(&DataKey::NextListingId, &1u64);
        env.storage().set(&DataKey::NextRetirementId, &1u64);
    }

    pub fn issue_credits(
        env: Env,
        project_id: u64,
        amount: u128,
    ) -> u64 {
        let verifier: Address = env.storage().get(&DataKey::Verifier).unwrap();
        verifier.require_auth();

        let mut project: Project = env.storage()
            .get(&DataKey::Project(project_id))
            .unwrap_or_else(|| panic!("project not found"));

        project.total_issued += amount;
        project.available += amount;
        env.storage().set(&DataKey::Project(project_id), &project);

        project_id
    }

    pub fn create_project(
        env: Env,
        name: String,
        region: String,
        project_type: String,
        vintage: u32,
    ) -> u64 {
        let admin: Address = env.storage().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut next_id: u64 = env.storage().get(&DataKey::NextProjectId).unwrap();
        let project = Project {
            owner: admin.clone(),
            name,
            region,
            project_type,
            vintage,
            total_issued: 0,
            available: 0,
        };
        env.storage().set(&DataKey::Project(next_id), &project);
        next_id += 1;
        env.storage().set(&DataKey::NextProjectId, &next_id);
        next_id - 1
    }

    pub fn list_credits(
        env: Env,
        project_id: u64,
        price_per_credit: u128,
        amount: u128,
    ) -> u64 {
        let caller: Address = env.storage().get(&DataKey::Admin).unwrap();
        caller.require_auth();

        let mut project: Project = env.storage()
            .get(&DataKey::Project(project_id))
            .unwrap_or_else(|| panic!("project not found"));

        if project.available < amount {
            panic!("insufficient credits");
        }
        project.available -= amount;
        env.storage().set(&DataKey::Project(project_id), &project);

        let mut next_id: u64 = env.storage().get(&DataKey::NextListingId).unwrap();
        let listing = Listing {
            project_id,
            seller: caller,
            price_per_credit,
            amount,
        };
        env.storage().set(&DataKey::NextListingId, &(next_id + 1));
        env.storage().set(&DataKey::Listing(next_id), &listing);
        next_id
    }

    pub fn retire_credits(
        env: Env,
        project_id: u64,
        amount: u128,
        reason: String,
    ) -> u64 {
        let caller = env.invoker();
        caller.require_auth();

        let mut project: Project = env.storage()
            .get(&DataKey::Project(project_id))
            .unwrap_or_else(|| panic!("project not found"));

        let mut balance: u128 = env.storage()
            .get(&DataKey::Balances(caller.clone()))
            .unwrap_or(0);
        if balance < amount {
            panic!("insufficient balance to retire");
        }
        balance -= amount;
        env.storage().set(&DataKey::Balances(caller.clone()), &balance);
        project.available -= amount;
        env.storage().set(&DataKey::Project(project_id), &project);

        let mut next_ret: u64 = env.storage().get(&DataKey::NextRetirementId).unwrap();
        let retirement = Retirement {
            retiree: caller,
            project_id,
            amount,
            reason,
            timestamp: env.ledger().timestamp(),
        };
        env.storage().set(&DataKey::Retirement(next_ret), &retirement);
        env.storage().set(&DataKey::NextRetirementId, &(next_ret + 1));
        next_ret
    }

    pub fn get_project(env: Env, project_id: u64) -> Project {
        env.storage()
            .get(&DataKey::Project(project_id))
            .unwrap_or_else(|| panic!("project not found"))
    }

    pub fn get_retirement(env: Env, retirement_id: u64) -> Retirement {
        env.storage()
            .get(&DataKey::Retirement(retirement_id))
            .unwrap_or_else(|| panic!("retirement not found"))
    }
}
