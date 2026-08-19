#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec};

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum TransactionStatus {
    Pending,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct CrossChainTransaction {
    pub id: u64,
    pub user: Address,
    pub asset: Address,
    pub amount: i128,
    pub to_chain: String,
    pub to_address: String,
    pub status: TransactionStatus,
    pub attestations: Vec<Address>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Validator {
    pub public_key: Address,
    pub is_active: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct BridgeState {
    pub next_transaction_id: u64,
    pub is_paused: bool,
    pub validator_count: u32,
    pub required_attestations: u32,
}

#[contract]
pub struct CrossChainBridge;

#[contractimpl]
impl CrossChainBridge {
    pub fn initialize(
        env: Env,
        admin: Address,
        validators: Vec<Address>,
        required_attestations: u32,
    ) {
        if env.storage().instance().has(&DataKey::State) {
            panic!("Bridge is already initialized");
        }

        env.storage().instance().set(&DataKey::Admin, &admin);

        let validator_count = validators.len();
        if required_attestations > validator_count {
            panic!("Required attestations cannot exceed validator count");
        }

        let state = BridgeState {
            next_transaction_id: 0,
            is_paused: false,
            validator_count,
            required_attestations,
        };

        env.storage().instance().set(&DataKey::State, &state);

        let mut validator_list: Vec<Validator> = Vec::new(&env);
        for public_key in validators.iter() {
            validator_list.push_back(Validator {
                public_key,
                is_active: true,
            });
        }
        env.storage().instance().set(&DataKey::Validators, &validator_list);
    }

    pub fn pause_bridge(env: Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut state: BridgeState = env.storage().instance().get(&DataKey::State).unwrap();
        state.is_paused = true;
        env.storage().instance().set(&DataKey::State, &state);
    }
}

    pub fn unpause_bridge(env: Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut state: BridgeState = env.storage().instance().get(&DataKey::State).unwrap();
        state.is_paused = false;
        env.storage().instance().set(&DataKey::State, &state);
    }

    pub fn lock_asset(
        env: Env,
        user: Address,
        asset: Address,
        amount: i128,
        to_chain: String,
        to_address: String,
    ) {
        user.require_auth();

        let mut state: BridgeState = env.storage().instance().get(&DataKey::State).unwrap();
        if state.is_paused {
            panic!("Bridge is currently paused");
        }

        let transaction_id = state.next_transaction_id;
        let transaction = CrossChainTransaction {
            id: transaction_id,
            user,
            asset,
            amount,
            to_chain,
            to_address,
            status: TransactionStatus::Pending,
            attestations: Vec::new(&env),
        };

        env.storage()
            .instance()
            .set(&DataKey::Transaction(transaction_id), &transaction);

        state.next_transaction_id += 1;
        env.storage().instance().set(&DataKey::State, &state);

        // TODO: Emit an event to notify validators
    }

    pub fn attest_transaction(env: Env, validator: Address, transaction_id: u64) {
        validator.require_auth();

        let state: BridgeState = env.storage().instance().get(&DataKey::State).unwrap();
        let validators: Vec<Validator> = env.storage().instance().get(&DataKey::Validators).unwrap();

        if !validators.iter().any(|v| v.public_key == validator && v.is_active) {
            panic!("Not an active validator");
        }

        let mut transaction: CrossChainTransaction = env
            .storage()
            .instance()
            .get(&DataKey::Transaction(transaction_id))
            .unwrap();

        if transaction.attestations.contains(&validator) {
            panic!("Validator has already attested to this transaction");
        }

        transaction.attestations.push_back(validator);

        if transaction.attestations.len() >= state.required_attestations {
            transaction.status = TransactionStatus::Completed;
            // TODO: Mint wrapped tokens on the destination chain
        }

        env.storage()
            .instance()
            .set(&DataKey::Transaction(transaction_id), &transaction);
    }

    pub fn add_validator(env: Env, new_validator: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut validators: Vec<Validator> = env.storage().instance().get(&DataKey::Validators).unwrap();
        if validators.iter().any(|v| v.public_key == new_validator) {
            panic!("Validator already exists");
        }

        validators.push_back(Validator {
            public_key: new_validator,
            is_active: true,
        });

        let mut state: BridgeState = env.storage().instance().get(&DataKey::State).unwrap();
        state.validator_count += 1;

        env.storage().instance().set(&DataKey::Validators, &validators);
        env.storage().instance().set(&DataKey::State, &state);
    }

    pub fn remove_validator(env: Env, validator_to_remove: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut validators: Vec<Validator> = env.storage().instance().get(&DataKey::Validators).unwrap();
        let initial_len = validators.len();

        validators.retain(|v| v.public_key != validator_to_remove);

        if validators.len() == initial_len {
            panic!("Validator not found");
        }

        let mut state: BridgeState = env.storage().instance().get(&DataKey::State).unwrap();
        state.validator_count -= 1;

        env.storage().instance().set(&DataKey::Validators, &validators);
        env.storage().instance().set(&DataKey::State, &state);
    }
}

#[contracttype]
pub enum DataKey {
    State,
    Validators,
    Transaction(u64),
    Admin,
}