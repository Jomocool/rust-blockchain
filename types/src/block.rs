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
// 定义一个Block结构体，用于表示区块链中的一个区块
// 该结构体派生了Serialize、Deserialize、Debug和Clone trait，分别用于序列化、反序列化、调试打印和深拷贝
// 使用serde属性，指定在序列化时将所有字段名转换为snake_case格式，在反序列化时也使用snake_case格式
pub struct Block {
    // 区块编号，使用U64类型表示
    pub number: U64,
    // 区块哈希值，可能为空，使用Option类型表示
    // 当值为None时，序列化时将跳过该字段
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<H256>,
    // 区块的父哈希值，用于链接区块
    pub parent_hash: H256,
    // 区块中的交易列表，使用Vec集合表示
    pub transactions: Vec<Transaction>,
    // 交易根哈希值，用于快速验证交易的完整性
    pub transactions_root: H256,
    // 状态根哈希值，用于快速验证区块状态的完整性
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

    /// 创建一个创世块（Genesis Block）
    ///
    /// 创世块是区块链中的第一个块，它具有以下特点：
    /// - 索引为0（`U64::zero()`）
    /// - 前一个块的哈希值为0（`H256::zero()`），因为它是第一个块，没有前一个块
    /// - 交易列表为空（`vec![]`），表示没有交易数据
    /// - Merkle树的根哈希值为0（`H256::zero()`），由于没有交易，因此没有Merkle树
    ///
    /// 返回值:
    /// - Result<Self>: 返回一个结果，包含成功创建的创世块实例或错误
    pub fn genesis() -> Result<Self> {
        Self::new(U64::zero(), H256::zero(), vec![], H256::zero())
    }
}
