#![cfg(test)]

use crate::LendingPoolClient;

use soroban_sdk::{Address, Env};

pub fn register_test_contract(e: &Env) -> Address {
    e.register_contract(None, crate::LendingPool {})
}

pub struct LendingPool {
    env: Env,
    contract_id: Address,
}

impl LendingPool {
    #[must_use]
    pub fn client(&self) -> LendingPoolClient {
        LendingPoolClient::new(&self.env, &self.contract_id)
    }

    #[must_use]
    pub fn new(env: &Env, contract_id: Address) -> Self {
        Self {
            env: env.clone(),
            contract_id,
        }
    }
}
