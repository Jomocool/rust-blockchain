use crate::bytes::Bytes;
use ethereum_types::{Address, U256};
use serde::{Deserialize, Serialize};
pub type Account = Address;

/// AccountData 结构体用于存储账户的相关数据
/// 包括 nonce（用于防止重放攻击的计数器），
/// balance（账户余额），以及 code_hash（账户代码的哈希值，用于识别合约账户）
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct AccountData {
    pub nonce: U256,
    pub balance: U256,
    pub code_hash: Option<Bytes>,
}

impl AccountData {
    /// 创建一个新的 AccountData 实例
    ///
    /// 参数:
    ///   - code_hash: 可选的字节序列，用于标识合约账户的代码哈希
    ///
    /// 返回值:
    ///   返回一个初始化了 code_hash 的 AccountData 实例，nonce 和 balance 初始化为零
    pub fn new(code_hash: Option<Bytes>) -> Self {
        AccountData {
            nonce: U256::zero(),
            balance: U256::from(10000),
            code_hash,
        }
    }

    /// 判断账户是否为合约账户
    ///
    /// 返回值:
    ///   如果账户有 code_hash，则返回 true，表示这是一个合约账户；
    ///   否则返回 false，表示这不是一个合约账户
    pub fn is_contract(&self) -> bool {
        self.code_hash.is_some()
    }
}
