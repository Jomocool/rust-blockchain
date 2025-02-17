//! # Transactions
//!
//! Interact with Ethereum transactions.
use crate::error::Result;
use crate::Web3;
use ethereum_types::H256;
use jsonrpsee::rpc_params;
use serde_json::to_value;
use types::bytes::Bytes;
use types::transaction::{TransactionReceipt, TransactionRequest};

impl Web3 {
    /// Create a new message call transaction or deploy a contract.
    pub async fn send(&self, transaction_request: TransactionRequest) -> Result<H256> {
        let transaction_request = to_value(&transaction_request)?;
        let params = rpc_params![transaction_request];
        let response = self.send_rpc("eth_sendTransaction", params).await?;
        let tx_hash: H256 = serde_json::from_value(response)?;

        Ok(tx_hash)
    }

    /// Send a raw transaction
    pub async fn send_raw(&self, transaction_request: Bytes) -> Result<H256> {
        let transaction_request = to_value(&transaction_request)?;
        let params = rpc_params![transaction_request];
        let response = self.send_rpc("eth_sendRawTransaction", params).await?;
        let tx_hash: H256 = serde_json::from_value(response)?;

        Ok(tx_hash)
    }

    /// Retrieve a transaction receipt by transaction hash.
    pub async fn transaction_receipt(&self, tx_hash: H256) -> Result<TransactionReceipt> {
        let tx_hash = to_value(tx_hash)?;
        let params = rpc_params![tx_hash];
        let response = self.send_rpc("eth_getTransactionReceipt", params).await?;
        let receipt = serde_json::from_value(response)?;

        Ok(receipt)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::helpers::tests::{
        deploy_contract, increment_account_1_nonce, web3, ACCOUNT_1, ACCOUNT_1_NONCE, ACCOUNT_2,
    };
    use ethereum_types::U256;
    use std::time::Duration;
    use tokio::time::sleep;
    use types::{account::Account, transaction::Transaction};
    use utils::crypto::keypair;

    async fn transaction() -> Transaction {
        let nonce = increment_account_1_nonce().await;
        Transaction::new(
            *ACCOUNT_1,
            Some(*ACCOUNT_2),
            U256::from(10),
            Some(nonce),
            None,
        )
        .unwrap()
    }

    async fn function_call_transaction(contract_account: Account, data: Bytes) -> Transaction {
        let nonce = increment_account_1_nonce().await;
        Transaction::new(
            *ACCOUNT_1,
            Some(contract_account),
            U256::from(10),
            Some(nonce),
            Some(data),
        )
        .unwrap()
    }

    pub async fn send_transaction() -> Result<H256> {
        let transaction_request: TransactionRequest = transaction().await.into();
        web3().send(transaction_request).await
    }

    #[tokio::test]
    async fn it_sends_a_transaction() {
        let response = send_transaction().await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn it_gets_a_transaction_receipt() {
        let tx_hash = send_transaction().await.unwrap();

        sleep(Duration::from_millis(2000)).await;

        let response = web3().transaction_receipt(tx_hash).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn it_sends_a_raw_transfer_transaction() {
        let (secret_key, _) = keypair();
        let transaction = transaction().await;
        let signed_transaction = web3().sign_transaction(transaction, secret_key).unwrap();
        let encoded = bincode::serialize(&signed_transaction).unwrap();
        let response = web3().send_raw(encoded.into()).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn it_sends_a_raw_contract_call_transaction() {
        let (secret_key, _) = keypair();
        let tx_hash = deploy_contract(false).await;

        sleep(Duration::from_millis(1000)).await;

        let receipt = web3().transaction_receipt(tx_hash).await.unwrap();
        let contract_address = receipt.contract_address.unwrap();
        let function_call = bincode::serialize(&(
            "construct",
            vec!["String", "Rust Coin 1", "String", "RustCoin1"],
        ))
        .unwrap();
        let transaction = function_call_transaction(contract_address, function_call.into()).await;
        let signed_transaction = web3().sign_transaction(transaction, secret_key).unwrap();
        let encoded = bincode::serialize(&signed_transaction).unwrap();
        let response = web3().send_raw(encoded.into()).await;
        assert!(response.is_ok());
    }
}
