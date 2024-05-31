#![no_std]
use soroban_sdk::{
    contract, contractimpl, contractmeta, contracttype, token, Address, Env, IntoVal, Val, Symbol,
};

mod allbridge {
    soroban_sdk::contractimport!(
        file = "../allbridge/bridge.wasm"
    );
}

mod events;
mod test;
mod testutils;

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    StartTime(u64),
    Owner,
    Token,
    Created,
    AllbridgeContract,
    DestinationPoolAddress,
    DestinationChainID,
    DestinationToken,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LendingAgreement {
    pub amount: u128,
    pub address: Address,
    pub currency: Symbol,
    pub timeperiod: u64,
    pub starttime: u64,
}


#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum State {
    Running = 0,
    Expired = 1,
    Cancelled = 2,
}

impl IntoVal<Env, Val> for State {
    fn into_val(&self, env: &Env) -> Val {
        (*self as u32).into_val(env)
    }
}

fn get_ledger_timestamp(e: &Env) -> u64 {
    e.ledger().timestamp()
}

fn get_owner(e: &Env) -> Address {
    e.storage()
        .instance()
        .get::<_, Address>(&DataKey::Owner)
        .expect("not initialized")
}


fn get_created(e: &Env) -> u64 {
    e.storage()
        .instance()
        .get::<_, u64>(&DataKey::Created)
        .expect("not initialized")
}

fn get_token(e: &Env) -> Address {
    e.storage()
        .instance()
        .get::<_, Address>(&DataKey::Token)
        .expect("not initialized")
}

fn get_allbridge_contract(e: &Env) -> Address {
    e.storage()
        .instance()
        .get::<_, Address>(&DataKey::AllbridgeContract)
        .expect("not initialized")
}

fn get_agreement(e: &Env, starttime: u64) -> LendingAgreement {
    e.storage()
        .instance()
        .get::<_, LendingAgreement>(&DataKey::StartTime(starttime))
        .expect("not initialized")
}

fn get_destination_pool_address(e: &Env) -> Address {
    e.storage()
        .instance()
        .get::<_, Address>(&DataKey::DestinationPoolAddress)
        .expect("not initialized")
}

fn get_destination_chain_id(e: &Env) -> Address {
    e.storage()
        .instance()
        .get::<_, Address>(&DataKey::DestinationChainID)
        .expect("not initialized")
}

fn get_destination_token(e: &Env) -> Address {
    e.storage()
        .instance()
        .get::<_, Address>(&DataKey::DestinationToken)
        .expect("not initialized")
}

fn get_state(e: &Env, agreement: &LendingAgreement) -> State {
    let expiry = agreement.starttime + agreement.timeperiod;
    let token_id = get_token(e);
    let current_timestamp = get_ledger_timestamp(e);

    if get_owner_cancelled(e) {
        return State::Cancelled;
    };
    if current_timestamp < expiry {
        return State::Running;
    };
    State::Expired
}

fn set_user_deposited(e: &Env, user: &Address, agreement: &LendingAgreement) {
    e.storage()
        .instance()
        .set(&DataKey::StartTime(agreement.starttime), agreement);
}

// Transfer tokens from the contract to the recipient
fn transfer(e: &Env, to: &Address, amount: &u128) {
    let token_contract_id = &get_token(e);
    let client = token::Client::new(e, token_contract_id);
    client.transfer(&e.current_contract_address(), to, amount);
}

// Metadata that is added on to the WASM custom section
contractmeta!(
    key = "Description",
    val = "LendingPool contract that allows users to deposit tokens and withdraw them"
);

#[contract]
struct LendingPool;

#[contractimpl]
#[allow(clippy::needless_pass_by_value)]
impl LendingPool {
    pub fn initialize(
        e: Env,
        owner: Address,
        token: Address,
        allbridge_contract: Address,
        destination_pool_address: BytesN<32>,
        destination_chain_id: u32,
        destination_token: BytesN<32>,
    ) {
        assert!(
            !e.storage().instance().has(&DataKey::Owner),
            "already initialized"
        );

        e.storage().instance().set(&DataKey::Owner, &owner);
        e.storage()
            .instance()
            .set(&DataKey::Created, &get_ledger_timestamp(&e));
        e.storage().instance().set(&DataKey::Token, &token);
        e.storage().instance().set(&DataKey::AllbridgeContract, &allbridge_contract);
        e.storage().instance().set(&DataKey::DestinationPoolAddress, &destination_pool_address);
        e.storage().instance().set(&DataKey::DestinationChainID, &destination_chain_id);
        e.storage().instance().set(&DataKey::DestinationToken, &destination_token);
    }

    pub fn owner(e: Env) -> Address {
        get_owner(&e)
    }

    pub fn created(e: Env) -> u64 {
        get_created(&e)
    }

    pub fn state(e: Env, agreement: LendingAgreement) -> u32 {
        get_state(&e, &agreement) as u32
    }

    pub fn token(e: Env) -> Address {
        get_token(&e)
    }

    pub fn time_remaining(e: Env, agreement: LendingAgreement) -> u64 {
        // TODO time remaining function to check how long left for an agreement
    }

    pub fn list_agreements(e: Env) -> [LendingAgreement] {
        // TODO list all agreements 
    }

    pub fn deposit(e: Env, 
        user: Address, 
        amount: u128, 
        currency: Symbol, 
        timeperiod: u64, 
        nonce: U256,
        gas_amount: u128,
        fee_token_amount: u128,
    ) {
        user.require_auth();
        assert!(amount > 0, "amount must be positive");

        let token_id = get_token(&e);
        let current_timestamp = get_ledger_timestamp(&e);
        let allbridge_contract: Address = get_allbridge_contract(&e);

        let agreement = LendingAgreement {
            amount,
            address: user,
            currency,
            timeperiod,
            starttime: current_timestamp,
        };

        set_user_deposited(&e, &get_owner(&e), &agreement);

        let client = token::Client::new(&e, &token_id);
        client.transfer(&user, &e.current_contract_address(), &amount);

        let allbridge_client = allbridge::Client::new(&env, &allbridge_contract);
        
        let sender = &e.current_contract_address();
        let recipient = get_destination_pool_address(&e);
        let destination_chain_id = get_destination_chain_id(&e);
        let receive_token = get_destination_token(&e);

        allbridge_client.swap_and_bridge(
            sender,
            token,
            &amount,
            recipient,
            destination_chain_id,
            receive_token,
            nonce,
            &gas_amount,
            &fee_token_amount,
        );

        // emit events
        events::agreement_created(&e, agreement.starttime);
    }

    pub fn withdraw(e: Env, to: Address, starttime: u64) {
        let agreement = e.storage()
            .instance()
            .get::<_, LendingAgreement>(&DataKey::StartTime(starttime))
            .expect("not initialized");

        let state = get_state(&e, &agreement);
        let owner = get_owner(&e);

        match state {
            State::Running => {
                panic!("agreement is not expired")
            }
            State::Cancelled => {
                assert!(
                    to == owner,
                    "can withraw only to the owner"
                );
                // TODO calculate the value based on formula
            }
            State::Expired => {
                assert!(
                    to != owner,
                    "can withraw only to the owner"
                );

                // Withdraw full amount
                let balance = get_user_deposited(&e, &to, starttime);
                set_user_deposited(&e, &to, &agreement);
                transfer(&e, &to, &balance);

                // emit events
                events::balance_withdrawn(&e, balance, starttime);
            }
        };
    }
}
