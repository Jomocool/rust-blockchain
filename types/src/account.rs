use crate::bytes::Bytes;
use ethereum_types::{Address, U256};
use serde::{Deserialize, Serialize};
pub type Account = Address;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct AccountData {
    pub nonce: U256,
    pub balance: U256,
    pub code_hash: Option<Bytes>,
}

impl AccountData {
    pub fn new(code_hash: Option<Bytes>) -> Self {
        AccountData {
            nonce: U256::zero(),
            balance: U256::zero(),
            code_hash,
        }
    }

    pub fn is_contract(&self) -> bool {
        self.code_hash.is_some()
    }
}
