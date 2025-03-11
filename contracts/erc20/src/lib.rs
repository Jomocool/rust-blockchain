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
        println!("name {}, symbol {}", name, symbol);
    }

    fn mint(account: String, amount: u64) {
        println!("account {}, amount {}", account, amount);
    }

    fn transfer(to: String, amount: u64) {
        println!("to {}, amount {}", to, amount);
    }
}
