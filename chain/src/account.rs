use std::sync::Arc;

use eth_trie::{EthTrie, Trie};
use ethereum_types::{H256, U256};
use types::account::{Account, AccountData};
use types::bytes::Bytes;
use utils::crypto::to_address;

use crate::helpers::{deserialize, serialize};
use crate::{
    error::{ChainError, Result},
    storage::Storage,
};

/// AccountStorage 结构体用于存储账户的相关信息。
/// 它使用 EthTrie 来管理存储数据，确保数据的高效检索和组织。
///
/// 字段:
/// - trie: 一个使用 Storage 作为底层数据结构的 EthTrie 实例。
///         它负责实际的数据存储和检索操作。
#[derive(Debug)]
pub(crate) struct AccountStorage {
    pub(crate) trie: EthTrie<Storage>,
}

impl AccountStorage {
    /// 创建一个新的AccountStorage实例
    pub(crate) fn new(storage: Arc<Storage>) -> Self {
        Self {
            trie: EthTrie::new(Arc::clone(&storage)),
        }
    }

    /// 插入或更新一个账户的数据
    pub(crate) fn upsert(&mut self, key: &Account, data: &AccountData) -> Result<()> {
        self.trie
            .insert(key.as_ref(), &serialize(&data)?)
            .map_err(|_| ChainError::StoragePutError(Storage::key_string(key)))
    }

    /// 添加或更新一个账户
    pub(crate) fn add_account(&mut self, key: &Account, data: &AccountData) -> Result<()> {
        self.upsert(key, data)
    }

    /// 添加一个合约账户
    pub fn add_contract_account(&mut self, key: &Account, data: Bytes) -> Result<Account> {
        let nonce = self.get_account(key)?.nonce;
        let serialized = bincode::serialize(&(key, nonce))?;
        let account = to_address(&serialized);
        let account_data = AccountData::new(Some(data));
        self.add_account(&account, &account_data)?;

        Ok(account)
    }

    /// 获取一个账户的数据
    pub(crate) fn get_account(&self, key: &Account) -> Result<AccountData> {
        let account = &self
            .trie
            .get(key.as_ref())
            .map_err(|_| ChainError::AccountNotFound(format!("Account {:?} not found", key)))?
            .ok_or_else(|| ChainError::StorageNotFound(Storage::key_string(key)))?;

        deserialize(account)
    }

    /// 获取所有账户
    pub(super) fn get_all_accounts(&self) -> Result<Vec<Account>> {
        let mut accounts = Vec::new();
        let mut iter = self.trie.iter();

        while let Some((key, _)) = iter.next() {
            accounts.push(Account::from_slice(&key));
        }

        Ok(accounts)
    }

    /// 增加一个账户的余额
    pub(crate) fn add_account_balance(&mut self, key: &Account, amount: U256) -> Result<()> {
        let mut account_data = self.get_account(key)?;
        account_data.balance += amount;
        self.upsert(key, &account_data)
    }

    /// 减少一个账户的余额
    pub(crate) fn subtract_account_balance(&mut self, key: &Account, amount: U256) -> Result<()> {
        let mut account_data = self.get_account(key)?;
        let balance = account_data.balance - amount;
        account_data.balance = std::cmp::max(U256::zero(), balance);
        self.upsert(key, &account_data)
    }

    /// 在账户之间转移余额
    pub(crate) fn transfer(&mut self, from: &Account, to: &Account, amount: U256) -> Result<()> {
        self.subtract_account_balance(from, amount)?;
        self.add_account_balance(to, amount)?;

        Ok(())
    }

    /// 更新账户的nonce值
    pub(crate) fn update_nonce(&mut self, key: &Account, nonce: U256) -> Result<U256> {
        let mut account_data = self.get_account(key)?;

        if nonce < account_data.nonce + 1 {
            return Err(ChainError::NonceTooLow(nonce.to_string(), key.to_string()));
        }

        if nonce > account_data.nonce + 1 {
            return Err(ChainError::NonceTooHigh(nonce.to_string(), key.to_string()));
        }

        account_data.nonce = nonce;
        self.upsert(key, &account_data)?;

        Ok(account_data.nonce)
    }

    /// 获取账户存储的根哈希值
    pub(crate) fn root_hash(&mut self) -> Result<H256> {
        let root_hash = self
            .trie
            .root_hash()
            .map_err(|e| ChainError::CannotCreateRootHash(format!("account_trie: {}", e)))?;

        Ok(H256::from_slice(root_hash.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::tests::STORAGE;
    use ethereum_types::H160;

    /// 创建一个新的账户存储实例
    ///
    /// 此函数通过克隆全局存储变量来初始化一个新的AccountStorage实例
    fn new_account_storage() -> AccountStorage {
        AccountStorage::new((*STORAGE).clone())
    }

    /// 向账户存储中添加一个新账户
    ///
    /// 参数:
    /// - account_storage: 账户存储的可变引用，用于添加账户
    ///
    /// 返回:
    /// - (AccountData, H160): 新增账户的数据及其对应的随机生成的密钥
    fn add_account(account_storage: &mut AccountStorage) -> (AccountData, H160) {
        let account_data = AccountData::new(None);
        let key = Account::random();
        account_storage.add_account(&key, &account_data).unwrap();

        (account_data, key)
    }

    /// 测试添加和获取账户的功能
    ///
    /// 此测试验证了新账户是否能成功添加到存储中，并且可以正确地通过其ID检索
    #[test]
    fn it_adds_and_gets_an_account() {
        let mut account_storage = new_account_storage();
        let (account_data, id) = add_account(&mut account_storage);
        let retrieved_account_data = account_storage.get_account(&id).unwrap();
        assert_eq!(retrieved_account_data, account_data);
    }

    /// 测试账户nonce的递增功能
    ///
    /// 此测试验证了账户的nonce是否能从零开始正确递增
    #[test]
    fn it_increments_a_nonce() {
        let mut account_storage = new_account_storage();
        let (_, id) = add_account(&mut account_storage);
        let retrieved_account_data = account_storage.get_account(&id).unwrap();
        assert_eq!(retrieved_account_data.nonce, U256::zero());

        let next_nonce = U256::from(1);
        account_storage.update_nonce(&id, next_nonce).unwrap();
        let retrieved_account_data = account_storage.get_account(&id).unwrap();
        assert_eq!(retrieved_account_data.nonce, next_nonce);
    }

    /// 测试在添加账户后根哈希是否发生变化
    ///
    /// 此测试验证了账户存储的根哈希在添加新账户后是否如预期那样发生变化
    #[test]
    fn root_hash_changes() {
        let mut account_storage = new_account_storage();
        let root_hash_1 = account_storage.root_hash().unwrap();
        let (_, _) = add_account(&mut account_storage);
        let root_hash_2 = account_storage.root_hash().unwrap();

        assert_ne!(root_hash_1, root_hash_2);
    }
}
