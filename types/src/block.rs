//! # Blocks
//!
//! Blocks are a fundamental aspect of the Ethereum blockchain.
//! A block can consist of many transactions.
//! Each block contains a hash of the parent block, which links blocks together.

use std::ops::Deref;

use ethereum_types::{H256, U64};
use serde::{Deserialize, Serialize};
use utils::crypto::hash;

use crate::{
    error::{Result, TypeError},
    helpers::hex_to_u64,
    transaction::Transaction,
};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename = "block_number")]
pub struct BlockNumber(pub U64);

impl Deref for BlockNumber {
    type Target = U64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<i32> for BlockNumber {
    fn from(value: i32) -> Self {
        BlockNumber(U64::from(value))
    }
}

impl TryFrom<&str> for BlockNumber {
    type Error = TypeError;

    fn try_from(value: &str) -> Result<Self> {
        Ok(BlockNumber(hex_to_u64(value.to_string())?))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all(serialize = "snake_case", deserialize = "snake_case"))]
pub struct Block {
    pub number: U64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<H256>,
    pub parent_hash: H256,
    pub transactions: Vec<Transaction>,
    pub transactions_root: H256,
    pub state_root: H256,
}

impl Block {
    pub fn new(
        number: U64,
        parent_hash: H256,
        transactions: Vec<Transaction>,
        state_root: H256,
    ) -> Result<Block> {
        let transactions_root = Transaction::root_hash(&transactions)?;
        let mut block = Block {
            number,
            hash: None,
            parent_hash,
            transactions,
            transactions_root,
            state_root,
        };

        let serialized = bincode::serialize(&block)?;
        let hash: H256 = hash(&serialized).into();
        block.hash = Some(hash);

        Ok(block)
    }

    pub fn block_hash(&self) -> Result<H256> {
        self.hash.ok_or(TypeError::MissingBlockHash)
    }

    pub fn genesis() -> Result<Self> {
        Self::new(U64::zero(), H256::zero(), vec![], H256::zero())
    }
}
