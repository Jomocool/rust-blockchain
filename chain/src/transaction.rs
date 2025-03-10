use crate::error::{ChainError, Result};

use dashmap::DashMap;
use ethereum_types::H256;
use std::collections::VecDeque;
use types::transaction::{Transaction, TransactionReceipt};

// 定义一个用于存储交易信息的结构体
#[derive(Debug)]
pub(crate) struct TransactionStorage {
    // 存储待处理交易的池
    pub(crate) mempool: VecDeque<Transaction>,
    // 存储交易哈希与其收据的映射
    pub(crate) receipts: DashMap<H256, TransactionReceipt>,
}

impl TransactionStorage {
    // 创建一个新的TransactionStorage实例
    pub(crate) fn new() -> Self {
        Self {
            mempool: VecDeque::new(),
            receipts: DashMap::new(),
        }
    }

    // 向交易池中发送一个交易
    pub(crate) fn send_transaction(&mut self, transaction: Transaction) {
        self.mempool.push_back(transaction);
    }

    // 根据交易哈希获取交易收据
    pub(crate) fn get_transaction_receipt(&self, hash: &H256) -> Result<TransactionReceipt> {
        let transaction_receipt = self
            .receipts
            .get(hash)
            .ok_or_else(|| ChainError::TransactionNotFound(hash.to_string()))?
            .value()
            .clone();

        Ok(transaction_receipt)
    }
}

// 单元测试配置
#[cfg(test)]
mod tests {
    use crate::blockchain::tests::{assert_receipt, new_transaction};
    use crate::helpers::tests::setup;

    use super::*;
    use types::account::Account;

    // 测试发送交易功能
    #[tokio::test]
    async fn sends_a_transaction() {
        let (blockchain, _, _) = setup().await;
        let mut transaction_storage = TransactionStorage::new();
        let transaction = new_transaction(Account::random(), blockchain.clone()).await;
        assert_eq!(transaction_storage.mempool.len(), 0);

        transaction_storage.send_transaction(transaction);
        assert_eq!(transaction_storage.mempool.len(), 1);
    }

    // 测试获取交易收据功能
    #[tokio::test]
    async fn gets_a_transaction_receipt() {
        let (blockchain, _, _) = setup().await;
        let to = Account::random();
        let transaction = new_transaction(to, blockchain.clone()).await;
        let transaction_hash = transaction.hash.unwrap();

        blockchain
            .lock()
            .await
            .transactions
            .lock()
            .await
            .send_transaction(transaction);

        assert_receipt(blockchain, transaction_hash).await;
    }
}
