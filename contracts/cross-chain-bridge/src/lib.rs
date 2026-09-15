#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, String, Vec, token};

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
    pub fee: i128,
    pub fee_address: Address,
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
        fee: i128,
        fee_address: Address,
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
            fee,
            fee_address,
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

    pub fn unpause_bridge(env: Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        let mut state: BridgeState = env.storage().instance().get(&DataKey::State).unwrap();
        state.is_paused = false;
        env.storage().instance().set(&DataKey::State, &state);
    }

    pub fn register_asset(env: Env, asset: Address, wrapped_asset: Address) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();

        env.storage()
            .instance()
            .set(&DataKey::AssetToWrapped(asset), &wrapped_asset);
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

        let fee_token_client = token::Client::new(&env, &asset);
        fee_token_client.transfer(&user, &state.fee_address, &state.fee);

        let amount_after_fee = amount - state.fee;

        let token_client = token::Client::new(&env, &asset);
        token_client.transfer(&user, &env.current_contract_address(), &amount_after_fee);

        let transaction_id = state.next_transaction_id;
        let transaction = CrossChainTransaction {
            id: transaction_id,
            user,
            asset,
            amount: amount_after_fee,
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

        env.events().publish(
            (String::from_str(&env, "lock_asset"), transaction.user.clone()),
            (transaction.id, transaction.amount),
        );
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

            let wrapped_asset = env
                .storage()
                .instance()
                .get(&DataKey::AssetToWrapped(transaction.asset.clone()))
                .unwrap();

            let token_client = token::Client::new(&env, &wrapped_asset);
            token_client.mint(&transaction.user, &transaction.amount);

            env.events().publish(
                (String::from_str(&env, "transaction_completed"), transaction.user.clone()),
                (transaction.id, transaction.amount),
            );
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

    pub fn emergency_withdraw(env: Env, user: Address, transaction_id: u64) {
        user.require_auth();

        let mut transaction: CrossChainTransaction = env
            .storage()
            .instance()
            .get(&DataKey::Transaction(transaction_id))
            .unwrap();

        if transaction.user != user {
            panic!("Not the transaction owner");
        }

        if transaction.status != TransactionStatus::Pending {
            panic!("Transaction is not in a pending state");
        }

        transaction.status = TransactionStatus::Cancelled;

        let token_client = token::Client::new(&env, &transaction.asset);
        token_client.transfer(
            &env.current_contract_address(),
            &user,
            &transaction.amount,
        );

        env.storage()
            .instance()
            .set(&DataKey::Transaction(transaction_id), &transaction);
    }
}

#[contracttype]
pub enum DataKey {
    State,
    Validators,
    Transaction(u64),
    Admin,
    AssetToWrapped(Address),
}