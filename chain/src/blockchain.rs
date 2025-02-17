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
use types::bytes::Bytes;
use types::transaction::{
    SignedTransaction, Transaction, TransactionKind, TransactionReceipt, TransactionRequest,
};

#[derive(Debug)]
pub(crate) struct BlockChain {
    pub(crate) accounts: AccountStorage,
    pub(crate) blocks: Vec<Block>,
    pub(crate) transactions: Arc<Mutex<TransactionStorage>>,
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
        let index = (block_number - 1).as_usize();
        let block = self
            .blocks
            .get(index)
            .ok_or_else(|| ChainError::BlockNotFound("current block".into()))?;

        Ok(block.to_owned())
    }

    pub(crate) fn parse_block_number(&self, block_number: &str) -> Result<BlockNumber> {
        if block_number == "latest" {
            Ok(BlockNumber(self.get_current_block()?.number))
        } else {
            Ok(block_number
                .try_into()
                .map_err(|_| ChainError::InvalidBlockNumber(block_number.into()))?)
        }
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

        // regenerate the transaction hash with the nonce in place
        let transaction_hash = transaction.hash()?;

        // add to the transaction mempool
        self.transactions.lock().await.send_transaction(transaction);

        Ok(transaction_hash)
    }

    pub(crate) async fn send_raw_transaction(&mut self, transaction: Bytes) -> Result<H256> {
        let signed_transaction: SignedTransaction = bincode::deserialize(&transaction)?;
        let transaction: Transaction = signed_transaction.clone().try_into()?;
        let transaction_hash = transaction.transaction_hash()?;

        Transaction::verify(signed_transaction, transaction.from).map_err(|e| {
            ChainError::TransactionNotVerified(format!("{}: {}", transaction_hash, e))
        })?;

        self.send_transaction(transaction.into()).await
    }

    pub(crate) async fn process_transactions(&mut self) -> Result<()> {
        // Bulk drain the current queue to fit into the new block
        // This is not safe as we lose transactions if a panic occurs
        // or if the program is halted
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
                    Err(error) => {
                        match error {
                            // The nonce is too high, add back to the mempool
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
                        }
                    }
                }
            }

            // update world state
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

            // now add the block number and hash to the receipts
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

    pub(crate) fn process_transaction<'a>(
        &mut self,
        transaction: &'a mut Transaction,
    ) -> Result<(&'a mut Transaction, TransactionReceipt)> {
        let mut contract_address: Option<Account> = None;
        let transaction_hash = transaction.transaction_hash()?;

        // ignore transactions without a nonce
        if let Some(nonce) = transaction.nonce {
            tracing::info!("Processing Transaction {:?}", transaction_hash);

            // create the `to` account if it doesn't exist
            if let Some(to) = transaction.to {
                self.accounts.add_empty_account(&to)?;
            }

            let kind = transaction.to_owned().kind()?;

            match kind {
                TransactionKind::Regular(from, to, value) => {
                    self.accounts.transfer(&from, &to, value)
                }
                TransactionKind::ContractDeployment(from, data) => {
                    contract_address = self.accounts.add_contract_account(&from, data).ok();
                    Ok(())
                }
                TransactionKind::ContractExecution(_from, to, data) => {
                    let code = self
                        .accounts
                        .get_account(&to)?
                        .code_hash
                        .ok_or_else(|| ChainError::NotAContractAccount(to.to_string()))?;
                    let (function, params): (&str, Vec<&str>) = bincode::deserialize(&data)?;

                    // call the function in the contract
                    runtime::contract::call_function(&code, function, &params)
                        .map_err(|e| ChainError::RuntimeError(to.to_string(), e.to_string()))
                }
            }?;

            // update the nonce
            self.accounts.update_nonce(&transaction.from, nonce)?;

            let transaction_receipt = TransactionReceipt {
                block_hash: None,
                block_number: None,
                contract_address,
                transaction_hash,
            };

            return Ok((transaction, transaction_receipt));
        }

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
    use utils::crypto::keypair;

    use super::*;
    use crate::{
        helpers::tests::{setup, ACCOUNT_1, STORAGE},
        transaction,
    };

    pub(crate) fn new_blockchain() -> BlockChain {
        BlockChain::new((*STORAGE).clone()).unwrap()
    }

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

    pub(crate) async fn process_transactions(blockchain: Arc<Mutex<BlockChain>>) {
        blockchain
            .lock()
            .await
            .process_transactions()
            .await
            .unwrap();
    }

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
    }

    pub(crate) async fn get_balance(blockchain: Arc<Mutex<BlockChain>>, account: &Account) -> U256 {
        blockchain
            .lock()
            .await
            .accounts
            .get_account(account)
            .unwrap()
            .balance
    }

    #[tokio::test]
    async fn creates_a_blockchain() {
        new_blockchain();
    }

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

    #[tokio::test]
    async fn send_a_raw_transaction() {
        let (blockchain, _, _) = setup().await;
        let to = Account::random();
        let (secret_key, _) = keypair();
        let transaction = new_transaction(to, blockchain.clone()).await;
        let signed_transaction = transaction.sign(secret_key).unwrap();
        let encoded = bincode::serialize(&signed_transaction).unwrap();
        let response = blockchain
            .lock()
            .await
            .send_raw_transaction(encoded.into())
            .await
            .unwrap();

        assert_receipt(blockchain.clone(), response).await;

        let balance = get_balance(blockchain, &to).await;
        assert_eq!(balance, U256::from(10));
    }
}
