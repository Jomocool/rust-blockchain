use std::collections::VecDeque;
use std::sync::Arc;

use crate::account::AccountStorage;
use crate::error::{ChainError, Result};
use crate::storage::Storage;
use crate::transaction::TransactionStorage;
use crate::world_state::WorldState;
use ethereum_types::{H256, U64};
use tokio::sync::Mutex;
use types::account::Account;
use types::block::{Block, BlockNumber};
use types::transaction::{Transaction, TransactionKind, TransactionReceipt, TransactionRequest};

#[derive(Debug)]
pub(crate) struct BlockChain {
    // AccountStorage用于存储区块链中的所有账户信息
    pub(crate) accounts: AccountStorage,
    // 存储区块链中的所有区块，Block类型代表区块链中的一个区块
    pub(crate) blocks: Vec<Block>,
    // 用于存储区块链中的所有交易，Arc<Mutex<_>>用于在多线程环境中安全地共享和修改数据
    pub(crate) transactions: Arc<Mutex<TransactionStorage>>,
    // WorldState代表系统的当前状态，存储了区块链中所有账户的状态信息
    pub(crate) world_state: WorldState,
}

impl BlockChain {
    pub(crate) fn new(storage: Arc<Storage>) -> Result<Self> {
        Ok(Self {
            accounts: AccountStorage::new(storage),
            blocks: vec![Block::genesis()?],
            transactions: Arc::new(Mutex::new(TransactionStorage::new())),
            world_state: WorldState::new(),
        })
    }

    pub(crate) fn get_current_block(&self) -> Result<Block> {
        let block = self
            .blocks
            .last()
            .ok_or_else(|| ChainError::BlockNotFound("current block".into()))?;

        Ok(block.to_owned())
    }

    pub(crate) fn get_block_by_number(&self, block_number: U64) -> Result<Block> {
        let index = block_number.as_usize();
        let block = self
            .blocks
            .get(index)
            .ok_or_else(|| ChainError::BlockNotFound("current block".into()))?;

        Ok(block.to_owned())
    }

    pub(crate) fn new_block(
        &mut self,
        transactions: Vec<Transaction>,
        state_trie: H256,
    ) -> Result<Block> {
        let current_block = self.get_current_block()?;
        let number = current_block.number + 1_u64;
        let parent_hash = current_block.block_hash()?;
        let block = Block::new(number, parent_hash, transactions, state_trie)?;

        self.blocks.push(block);

        self.get_block_by_number(number)
    }

    pub(crate) async fn send_transaction(
        &mut self,
        transaction_request: TransactionRequest,
    ) -> Result<H256> {
        let mut transaction: Transaction = transaction_request.try_into()?;
        let account = self.accounts.get_account(&transaction.from)?;
        let nonce = transaction.nonce.unwrap_or_else(|| account.nonce + 1_u64);

        transaction.nonce = Some(nonce);

        let transaction_hash = transaction.hash()?;

        self.transactions.lock().await.send_transaction(transaction);

        Ok(transaction_hash)
    }

    pub(crate) async fn process_transactions(&mut self) -> Result<()> {
        let transactions = self
            .transactions
            .lock()
            .await
            .mempool
            .drain(0..)
            .collect::<VecDeque<_>>();

        if !transactions.is_empty() {
            let mut receipts: Vec<TransactionReceipt> = vec![];
            let mut processed: Vec<Transaction> = vec![];

            tracing::info!("Processing {} transactions", transactions.len());

            for mut transaction in transactions.into_iter() {
                match self.process_transaction(&mut transaction) {
                    Ok((transaction, transaction_receipt)) => {
                        receipts.push(transaction_receipt);
                        processed.push(transaction.to_owned());
                    }
                    Err(error) => match error {
                        ChainError::NonceTooHigh(_, _) => {
                            tracing::warn!(
                                "Could not process transaction {:?}: {}",
                                transaction,
                                error
                            );
                            self.transactions
                                .lock()
                                .await
                                .mempool
                                .push_back(transaction);
                        }
                        _ => tracing::error!(
                            "Could not process transaction {:?}: {}",
                            transaction,
                            error
                        ),
                    },
                }
            }

            let state_trie = self.accounts.root_hash()?;
            self.world_state.update_state_trie(state_trie);

            tracing::info!("World State: state_trie {:?}", state_trie);

            let num_processed = processed.len();
            let block = self.new_block(processed, state_trie)?;

            tracing::info!(
                "Created block {} with {} transactions",
                block.number,
                num_processed
            );

            for mut receipt in receipts.into_iter() {
                receipt.block_number = Some(BlockNumber(block.number));
                receipt.block_hash = block.hash;

                self.transactions
                    .clone()
                    .lock()
                    .await
                    .receipts
                    .insert(receipt.transaction_hash, receipt);
            }

            let storage = self.transactions.lock().await;

            tracing::info!(
                "Transaction storage: mempool {:?}, receipts {:?}",
                storage.mempool.len(),
                storage.receipts.len()
            );
        }

        Ok(())
    }

    /// 处理交易函数
    ///
    /// 该函数负责处理不同类型的交易，包括常规转账、合约部署和合约执行
    /// 它会根据交易类型执行相应的操作，并生成交易收据
    ///
    /// 参数:
    /// - `transaction`: 一个可变的交易引用，表示需要处理的交易
    ///
    /// 返回值:
    /// - `Result<(&'a mut Transaction, TransactionReceipt)>`: 返回一个包含可变交易引用和交易收据的结果类型
    ///   如果处理成功，则包含交易和收据；如果处理失败，则包含相应的错误信息
    pub(crate) fn process_transaction<'a>(
        &mut self,
        transaction: &'a mut Transaction,
    ) -> Result<(&'a mut Transaction, TransactionReceipt)> {
        // 初始化合约地址为None，因为在处理交易时可能不会创建合约
        let mut contract_address: Option<Account> = None;
        // 获取交易哈希值
        let transaction_hash = transaction.transaction_hash()?;

        // 如果交易包含nonce，则开始处理交易
        if let Some(nonce) = transaction.nonce {
            // 记录交易处理信息
            tracing::info!("Processing Transaction {:?}", transaction_hash);

            // 判断目标账户是否存在，如果不存在返回错误
            if let Some(to) = transaction.to {
                if self.accounts.get_account(&to).is_err() {
                    return Err(ChainError::AccountNotFound(to.to_string()));
                }
            }

            // 获取交易类型
            let kind = transaction.to_owned().kind()?;

            // 根据交易类型处理交易
            match kind {
                // 处理常规转账交易
                TransactionKind::Regular(from, to, value) => {
                    self.accounts.transfer(&from, &to, value)
                }
                // 处理合约部署交易
                TransactionKind::ContractDeployment(from, data) => {
                    // 部署合约，并尝试获取合约地址
                    contract_address = self.accounts.add_contract_account(&from, data).ok();
                    Ok(())
                }
                // 处理合约执行交易
                TransactionKind::ContractExecution(_from, to, data) => {
                    // 获取合约账户的代码哈希
                    let code = self
                        .accounts
                        .get_account(&to)?
                        .code_hash
                        .ok_or_else(|| ChainError::NotAContractAccount(to.to_string()))?;
                    // 反序列化合约数据以获取函数和参数
                    let (function, params): (&str, Vec<&str>) = bincode::deserialize(&data)?;

                    // 调用合约函数
                    runtime::contract::call_function(&code, function, &params)
                        .map_err(|e| ChainError::RuntimeError(to.to_string(), e.to_string()))
                }
            }?;

            // 更新账户的nonce值
            self.accounts.update_nonce(&transaction.from, nonce)?;

            // 创建交易收据
            let transaction_receipt = TransactionReceipt {
                block_hash: None,
                block_number: None,
                contract_address,
                transaction_hash,
            };

            // 返回处理后的交易和交易收据
            return Ok((transaction, transaction_receipt));
        }

        // 如果交易不包含nonce，则返回错误
        Err(ChainError::MissingTransactionNonce(
            transaction_hash.to_string(),
        ))
    }

    pub(crate) async fn get_transaction_receipt(
        &mut self,
        transaction_hash: H256,
    ) -> Result<TransactionReceipt> {
        let transaction_receipt = self
            .transactions
            .lock()
            .await
            .get_transaction_receipt(&transaction_hash)?;

        Ok(transaction_receipt)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use ethereum_types::U256;
    use types::account::AccountData;

    use super::*;
    use crate::helpers::tests::{setup, ACCOUNT_1, STORAGE};

    /// 创建一个新的区块链实例
    pub(crate) fn new_blockchain() -> BlockChain {
        BlockChain::new((*STORAGE).clone()).unwrap()
    }

    /// 创建一个新的交易
    pub(crate) async fn new_transaction(
        to: Account,
        blockchain: Arc<Mutex<BlockChain>>,
    ) -> Transaction {
        let nonce = blockchain
            .lock()
            .await
            .accounts
            .get_account(&ACCOUNT_1)
            .unwrap_or(AccountData::new(None))
            .nonce
            + 1;

        let transaction =
            Transaction::new(*ACCOUNT_1, Some(to), U256::from(10), Some(nonce), None).unwrap();

        transaction
    }

    /// 处理交易
    pub(crate) async fn process_transactions(blockchain: Arc<Mutex<BlockChain>>) {
        blockchain
            .lock()
            .await
            .process_transactions()
            .await
            .unwrap();
    }

    /// 断言交易收据
    pub(crate) async fn assert_receipt(blockchain: Arc<Mutex<BlockChain>>, transaction_hash: H256) {
        process_transactions(blockchain.clone()).await;

        let receipt = blockchain
            .lock()
            .await
            .transactions
            .lock()
            .await
            .get_transaction_receipt(&transaction_hash)
            .unwrap();

        assert_eq!(receipt.transaction_hash, transaction_hash);
    }

    /// 获取账户余额
    pub(crate) async fn get_balance(blockchain: Arc<Mutex<BlockChain>>, account: &Account) -> U256 {
        blockchain
            .lock()
            .await
            .accounts
            .get_account(account)
            .unwrap()
            .balance
    }

    /// 测试创建区块链
    #[tokio::test]
    async fn creates_a_blockchain() {
        new_blockchain();
    }

    /// 测试创建和获取一个区块
    #[tokio::test]
    async fn creates_and_gets_a_block() {
        let (blockchain, _, _) = setup().await;
        let block_number = blockchain.lock().await.get_current_block().unwrap().number;
        let transaction = new_transaction(Account::random(), blockchain.clone()).await;
        let response = blockchain
            .lock()
            .await
            .new_block(vec![transaction], H256::zero());
        assert!(response.is_ok());

        let new_block_number = blockchain.lock().await.get_current_block().unwrap().number;
        assert_eq!(new_block_number, block_number + 1);
    }

    /// 测试发送交易
    #[tokio::test]
    async fn sends_a_transaction() {
        let (blockchain, _, _) = setup().await;
        let to = Account::random();
        let transaction = new_transaction(to, blockchain.clone()).await;
        let transaction_hash = blockchain
            .lock()
            .await
            .send_transaction(transaction.into())
            .await
            .unwrap();

        assert_receipt(blockchain.clone(), transaction_hash).await;

        let balance = get_balance(blockchain, &to).await;
        assert_eq!(balance, U256::from(10));
    }
}
