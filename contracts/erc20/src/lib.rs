use std::collections::HashMap;

wit_bindgen::generate!("erc20");

pub struct Erc20;

#[allow(dead_code)]
pub struct State {
    name: String,
    symbol: String,
    balances: HashMap<String, u64>,
}

export_contract!(Erc20);

impl Contract for Erc20 {
    fn construct(name: String, symbol: String) {
        println!(
            "construct called successfully, params: [ String, {}, String, {}]",
            name, symbol
        );
    }

    fn mint(account: String, amount: u64) {
        println!(
            "mint called successfully, params: [String, {}, U64, {}]",
            account, amount
        );
    }

    fn transfer(to: String, amount: u64) {
        println!(
            "transfer called successfully, params: [String, {}, U64, {}]",
            to, amount
        );
    }
}
