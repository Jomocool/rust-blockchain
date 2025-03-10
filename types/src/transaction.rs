use std::sync::Arc;

use crate::account::Account;
use crate::block::BlockNumber;
use crate::bytes::Bytes;
use crate::error::{Result, TypeError};
use eth_trie::{EthTrie, MemoryDB, Trie};
use ethereum_types::{Address, H160, H256, U256, U64};
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use utils::crypto::{
    hash, public_key_address, recover_public_key, sign_recovery, verify, Signature,
};
use utils::{PublicKey, RecoverableSignature, RecoveryId, SecretKey};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all(serialize = "camelCase", deserialize = "camelCase"))]
/// 代表一个交易的对象，包含了交易的相关信息。
///
/// 字段说明：
/// - `from`: 交易发起者的地址。
/// - `to`: 可选字段，代表交易接收者的地址。在合同部署交易中可能为空。
/// - `hash`: 可选字段，代表交易的哈希值。默认为空，当哈希值不存在时不会被序列化。
/// - `nonce`: 可选字段，代表交易的nonce值，用于确保交易的唯一性和顺序。
/// - `value`: 交易中转移的金额值。
/// - `data`: 可选字段，代表交易的数据部分，通常用于合同调用或创建。
/// - `gas`: 交易中使用的gas量。
/// - `gas_price`: 交易中使用的gas价格。
pub struct Transaction {
    pub from: Address,
    pub to: Option<Address>,
    /// 使用serde属性来默认处理这个字段，并在序列化时如果值为None则跳过。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<H256>,
    pub nonce: Option<U256>,
    pub value: U256,
    pub data: Option<Bytes>,
    pub gas: U256,
    pub gas_price: U256,
}

/// 交易类型枚举，用于区分不同的交易种类
pub enum TransactionKind {
    /// 普通交易，包含交易双方地址和交易金额
    Regular(Address, Address, U256),
    /// 合约部署交易，包含部署者地址和合约字节码
    ContractDeployment(Address, Bytes),
    /// 合约执行交易，包含执行者地址、合约地址和执行数据
    ContractExecution(Address, Address, Bytes),
}

impl Transaction {
    pub fn new(
        from: Account,
        to: Option<Account>,
        value: U256,
        nonce: Option<U256>,
        data: Option<Bytes>,
    ) -> Result<Self> {
        let mut transaction = Self {
            from,
            to,
            value,
            nonce,
            hash: None,
            data,
            gas: U256::from(10),
            gas_price: U256::from(10),
        };

        transaction.hash()?;

        Ok(transaction)
    }

    pub fn hash(&mut self) -> Result<H256> {
        let serialized = bincode::serialize(&self)?;
        let hash: H256 = hash(&serialized).into();
        self.hash = Some(hash);

        self.transaction_hash()
    }

    pub fn transaction_hash(&self) -> Result<H256> {
        self.hash.ok_or(TypeError::MissingTransactionHash)
    }

    pub fn kind(self) -> Result<TransactionKind> {
        match (self.from, self.to, self.data) {
            (from, Some(to), None) => Ok(TransactionKind::Regular(from, to, self.value)),
            (from, None, Some(data)) => Ok(TransactionKind::ContractDeployment(from, data)),
            (from, Some(to), Some(data)) => Ok(TransactionKind::ContractExecution(from, to, data)),
            _ => Err(TypeError::InvalidTransaction("kind".into())),
        }
    }

    /// 使用给定的密钥对交易进行签名
    /// 
    /// 该方法首先将交易信息序列化为字节流，然后使用密钥对其进行签名
    /// 签名过程产生一个可恢复的签名，从中我们可以提取出签名的v、r、s值
    /// 最后，将这些签名值连同原始交易数据一起封装成一个签名交易对象，并返回
    /// 
    /// # 参数
    /// * `key` - 用于签名交易的密钥
    /// 
    /// # 返回
    /// 如果签名成功，返回一个`SignedTransaction`对象，包含签名信息和原始交易数据
    /// 如果签名过程中出现错误，返回相应的错误
    pub fn sign(&self, key: SecretKey) -> Result<SignedTransaction> {
        // 将交易信息序列化为字节流
        let encoded = bincode::serialize(&self)?;
        // 使用密钥对序列化的交易信息进行签名，产生一个可恢复的签名
        let recoverable_signature = sign_recovery(&encoded, &key)?;
        // 将可恢复的签名序列化为紧凑形式，获取签名的字节表示
        let (_, signature_bytes) = recoverable_signature.serialize_compact();
        // 从可恢复的签名中提取出v、r、s值
        let Signature { v, r, s } = recoverable_signature.into();
        // 计算签名的哈希值，作为交易的标识
        let transaction_hash = hash(&signature_bytes).into();
    
        // 创建签名交易对象
        let signed_transaction = SignedTransaction {
            v,
            r,
            s,
            raw_transaction: encoded.into(),
            transaction_hash,
        };
    
        // 返回签名交易对象
        Ok(signed_transaction)
    }

    /// 验证签名的交易是否合法
    ///
    /// 该函数主要负责验证一个已签名的交易是否合法，通过检查交易的签名和发送方地址
    /// # 参数
    /// * `signed_transaction` - 已签名的交易，用于提取消息、恢复ID和签名字节
    /// * `address` - 发送方的地址，用于与从签名中恢复的公钥地址进行匹配
    /// # 返回值
    /// 返回一个布尔值，表示交易的合法性（`true` 表示合法，`false` 表示不合法）
    pub fn verify(signed_transaction: SignedTransaction, address: Address) -> Result<bool> {
        // 从已签名的交易中提取消息、恢复ID和签名字节
        let (message, recovery_id, signature_bytes) = Self::recover_pieces(signed_transaction)?;
    
        // 根据消息、签名字节和恢复ID恢复公钥
        let key = recover_public_key(&message, &signature_bytes, recovery_id.to_i32())?;
    
        // 验证消息的签名是否与恢复的公钥匹配
        let verified = verify(&message, &signature_bytes, &key)?;
    
        // 检查恢复的公钥地址是否与提供的发送方地址匹配
        let addresses_match = address == public_key_address(&key);
    
        // 返回签名验证和地址匹配的逻辑与结果
        Ok(verified && addresses_match)
    }

    /// 从已签名的交易中恢复发送者的地址
    ///
    /// # 参数
    ///
    /// * `signed_transaction` - 已签名的交易，从中提取签名和消息以恢复公钥
    ///
    /// # 返回
    ///
    /// * `Result<H160>` - 发送者的地址，如果恢复成功，则为包含地址的Ok结果，否则为错误
    pub fn recover_address(signed_transaction: SignedTransaction) -> Result<H160> {
        // 从已签名的交易中恢复公钥
        let key = Self::recover_public_key(signed_transaction)?;
        // 使用恢复的公钥获取对应的地址
        let address = public_key_address(&key);
    
        // 返回成功恢复的地址
        Ok(address)
    }

    /// 从已签名的交易中恢复公钥
    ///
    /// 此函数通过已签名交易中的信息来恢复发送者的公钥它首先提取出消息、恢复ID和签名字节，
    /// 然后使用这些信息来计算并验证公钥
    ///
    /// # 参数
    ///
    /// * `signed_transaction` - 已签名的交易，其中包含恢复公钥所需的信息
    ///
    /// # 返回
    ///
    /// 如果成功恢复公钥，则返回一个包含公钥的Result如果恢复过程中发生错误，则返回一个错误
    pub fn recover_public_key(signed_transaction: SignedTransaction) -> Result<PublicKey> {
        // 从已签名的交易中提取出消息、恢复ID和签名字节
        let (message, recovery_id, signature_bytes) = Self::recover_pieces(signed_transaction)?;
    
        // 使用提取的信息来恢复公钥
        let key = recover_public_key(&message, &signature_bytes, recovery_id.to_i32())?;
    
        // 返回恢复的公钥
        Ok(key)
    }

    /// 从签名的交易中恢复出消息、恢复ID和签名字节
    ///
    /// 该函数的主要作用是从一个签名的交易中提取出必要的信息，包括消息本身、恢复ID以及签名的字节表示
    /// 这些信息可以用于进一步的加密操作或验证过程
    ///
    /// # 参数
    ///
    /// * `signed_transaction` - 一个签名过的交易，从中我们提取信息
    ///
    /// # 返回值
    ///
    /// 函数返回一个结果，包含一个元组：
    /// - 原始消息的字节向量
    /// - 恢复ID，用于帮助恢复公钥
    /// - 签名的64字节紧凑表示
    ///
    /// # 错误处理
    ///
    /// 如果无法从签名中恢复出可恢复的签名，函数将返回一个错误
    fn recover_pieces(
        signed_transaction: SignedTransaction,
    ) -> Result<(Vec<u8>, RecoveryId, [u8; 64])> {
        // 获取原始消息，这里是签名交易的原始交易信息
        let message = signed_transaction.raw_transaction.to_owned();
        
        // 将签名交易转换为签名对象
        let signature: Signature = signed_transaction.into();
        
        // 尝试将签名转换为可恢复的签名，这可能失败，因此使用try_into并返回可能的错误
        let recoverable_signature: RecoverableSignature = signature.try_into()?;
        
        // 将可恢复的签名序列化为紧凑形式，同时提取恢复ID
        let (recovery_id, signature_bytes) = recoverable_signature.serialize_compact();
    
        // 返回包含消息、恢复ID和签名字节的结果
        Ok((message.to_vec(), recovery_id, signature_bytes))
    }

    fn to_trie(transactions: &[Transaction]) -> Result<EthTrie<MemoryDB>> {
        let memdb = Arc::new(MemoryDB::new(true));
        let mut trie = EthTrie::new(memdb);

        transactions.iter().try_for_each(|transaction| {
            trie.insert(
                transaction.transaction_hash()?.as_bytes(),
                bincode::serialize(&transaction)?.as_slice(),
            )
            .map_err(|e| TypeError::TrieError(format!("Error inserting transactions: {}", e)))
        })?;

        Ok(trie)
    }

    pub fn root_hash(transactions: &[Transaction]) -> Result<H256> {
        let mut trie = Self::to_trie(transactions)?;
        let root_hash = trie
            .root_hash()
            .map_err(|e| TypeError::TrieError(format!("Error calculating root hash: {}", e)))?;

        Ok(H256::from_slice(root_hash.as_bytes()))
    }
}

/// 表示一个已签名的交易。
///
/// 这个结构体包含了签名交易的所有必要信息，包括签名的v、r、s值，原始交易数据以及交易的哈希值。
///
/// 字段说明：
/// - `v`: 签名的恢复ID。
/// - `r`: ECDSA签名的一部分,它是由随机数 k 和交易数据的哈希值共同决定的。
/// - `s`: ECDSA签名的另一部分,是通过私钥 d、随机数 k、交易数据的哈希值 z 以及 r 计算得出的。
/// - `raw_transaction`: 交易的原始字节数据。
/// - `transaction_hash`: 交易的哈希值，用于唯一标识该交易。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SignedTransaction {
    pub v: u64,
    pub r: H256,
    pub s: H256,
    pub raw_transaction: Bytes,
    pub transaction_hash: H256,
}

impl From<SignedTransaction> for Signature {
    fn from(value: SignedTransaction) -> Self {
        Signature {
            v: value.v,
            r: value.r,
            s: value.s,
        }
    }
}

impl TryInto<Transaction> for SignedTransaction {
    type Error = TypeError;

    fn try_into(self) -> Result<Transaction> {
        bincode::deserialize(&self.raw_transaction)
            .map_err(|e| TypeError::EncodingDecodingError(e.to_string()))
    }
}

#[skip_serializing_none]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(serialize = "camelCase", deserialize = "camelCase"))]
pub struct TransactionRequest {
    pub data: Option<Bytes>,
    pub gas: U256,
    pub gas_price: U256,
    pub from: Option<Address>,
    pub to: Option<Address>,
    pub value: Option<U256>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nonce: Option<U256>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r: Option<U256>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s: Option<U256>,
}

impl From<Transaction> for TransactionRequest {
    fn from(value: Transaction) -> TransactionRequest {
        TransactionRequest {
            from: Some(value.from),
            to: value.to,
            value: Some(value.value),
            data: value.data,
            gas: value.gas,
            gas_price: value.gas_price,
            nonce: value.nonce,
            r: None,
            s: None,
        }
    }
}

impl TryInto<Transaction> for TransactionRequest {
    type Error = TypeError;

    fn try_into(self) -> Result<Transaction> {
        let value = self.value.unwrap_or(U256::zero());
        let from = self.from.unwrap_or(H160::zero());
        Transaction::new(from, self.to, value, self.nonce, self.data)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all(serialize = "camelCase", deserialize = "camelCase"))]
pub struct TransactionReceipt {
    pub block_hash: Option<H256>,
    pub block_number: Option<BlockNumber>,
    pub contract_address: Option<H160>,
    pub transaction_hash: H256,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all(serialize = "snake_case", deserialize = "camelCase"))]
pub struct Log {
    pub address: H160,
    pub block_hash: Option<H256>,
    pub block_number: Option<U64>,
    pub data: Bytes,
    pub log_index: Option<U256>,
    pub log_type: Option<String>,
    pub removed: Option<bool>,
    pub topics: Vec<H256>,
    pub transaction_hash: Option<H256>,
    pub transaction_index: Option<String>,
    pub transaction_log_index: Option<U256>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethereum_types::U256;
    use std::{convert::From, str::FromStr};
    use utils::crypto::{keypair, public_key_address};

       /// 创建一个新的交易实例
    /// 
    /// 该函数初始化了一个从固定地址到另一个固定地址的交易，交易金额为1个以太币
    /// 主要用于测试和示例场景，以确保交易对象的正确创建
    pub(crate) fn new_transaction() -> Transaction {
        // 初始化交易发送方地址
        let from = H160::from_str("0x4a0d457e884ebd9b9773d172ed687417caac4f14").unwrap();
        // 初始化交易接收方地址
        let to = H160::from_str("0x6b78fa07883d5c5b527da9828ac77f5aa5a61d3b").unwrap();
        // 初始化交易金额
        let value = U256::from(1u64);
    
        // 创建并返回交易对象
        Transaction::new(from, Some(to), value, None, None).unwrap()
    }
    
    /// 测试从签名交易中恢复地址的功能
    /// 
    /// 该测试函数验证了从签名交易中恢复出的地址是否与使用公钥计算出的地址一致
    #[test]
    fn it_recovers_an_address_from_a_signed_transaction() {
        // 生成密钥对
        let (secret_key, public_key) = keypair();
        // 创建交易
        let transaction = new_transaction();
        // 签名交易
        let signed = transaction.sign(secret_key).unwrap();
        // 从签名中恢复地址
        let recovered = Transaction::recover_address(signed).unwrap();
    
        // 验证恢复的地址与公钥计算出的地址是否一致
        assert_eq!(recovered, public_key_address(&public_key));
    }
    
    /// 测试验证签名交易的功能
    /// 
    /// 该测试函数验证了一个签名交易是否能被正确验证
    #[test]
    fn it_verifies_a_signed_transaction() {
        // 生成密钥对
        let (secret_key, public_key) = keypair();
        // 创建交易并将发送方地址设置为公钥对应的地址
        let mut transaction = new_transaction();
        transaction.from = public_key_address(&public_key);
        // 签名交易
        let signed = transaction.sign(secret_key).unwrap();
        // 验证签名
        let verifies = Transaction::verify(signed, transaction.from).unwrap();
    
        // 断言验证结果为真
        assert!(verifies);
    }
    
    /// 测试计算交易树的根哈希值
    /// 
    /// 该测试函数验证了给定一组交易后计算出的Merkle树根哈希值是否符合预期
    #[test]
    fn root_hash() {
        // 创建两个交易
        let transaction_1 = new_transaction();
        let transaction_2 = new_transaction();
        // 计算交易的Merkle树根哈希值
        let root = Transaction::root_hash(&vec![transaction_1, transaction_2]).unwrap();
        // 预期的根哈希值
        let expected =
            H256::from_str("0xa3b8c35bab6501806ed681220afe26a0d46774a6aa56d044b0f6aef0f3f0d682")
                .unwrap();
        // 验证计算出的根哈希值与预期值是否一致
        assert_eq!(root, expected);
    }
}
