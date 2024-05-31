use soroban_sdk::{vec, Env, Symbol};

pub(crate) fn agreement_created(e: &Env, agreement_start: u64) {
    let topics = (Symbol::new(e, "agreement_created"),);
    e.events().publish(topics, agreement_start);
}

pub(crate) fn balance_withdrawn(e: &Env, amount: u128, agreement_start: u64) {
    let topics = (Symbol::new(e, "balance_withdrawn"),);
    let event_payload = vec![e, amount, agreement_start];
    e.events().publish(topics, event_payload);
}