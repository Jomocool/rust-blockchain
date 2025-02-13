use crate::error::{Result, Web3Error};
use crate::Web3;
use ethereum_types::U256;
use jsonrpsee::rpc_params;
use types::account::Account;
use types::block::BlockNumber;
use types::helpers::to_hex;
use types::transaction::{SignedTransaction, Transaction};
use utils::crypto::SecretKey;

impl Web3 {
    /// Retrieve the eth balance for an accout at a given block.
    pub async fn get_balance_by_block(
        &self,
        address: Account,
        block_number: Option<BlockNumber>,
    ) -> Result<U256> {
        let block_number = Web3::get_hex_blocknumber(block_number);
        let params = rpc_params![to_hex(address), block_number];
        let response = self.send_rpc("eth_getBalanceByBlock", params).await?;
        let balance: U256 = serde_json::from_value(response)?;

        Ok(balance)
    }

    pub async fn get_balance(&self, address: Account) -> Result<U256> {
        let balance: U256 = self.get_balance_by_block(address, None).await?;

        Ok(balance)
    }

    pub fn sign_transaction(
        &self,
        transaction: Transaction,
        key: SecretKey,
    ) -> Result<SignedTransaction> {
        let signed_transaction = transaction.sign(key).map_err(|e| {
            Web3Error::TransactionSigningError(format!("{:?} {}", transaction.hash, e))
        })?;
        Ok(signed_transaction)
    }

    /// Retrieve the eth balance for an accout at a given block.
    pub async fn get_transaction_count(&self, address: Account) -> Result<U256> {
        let params = rpc_params![to_hex(address)];
        let response = self.send_rpc("eth_getTransactionCount", params).await?;
        let balance: U256 = serde_json::from_value(response)?;

        Ok(balance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::tests::{web3, ACCOUNT_1};

    #[tokio::test]
    async fn it_gets_a_balance() {
        let account = *ACCOUNT_1;
        let response = web3().get_balance(account).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn it_gets_a_balance_by_block() {
        // crate::transaction::tests::send_transaction().await.unwrap();
        let account = *ACCOUNT_1;
        let response = web3()
            .get_balance_by_block(account, Some(BlockNumber(0.into())))
            .await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn it_gets_a_transaction_count() {
        let account = *ACCOUNT_1;
        let response = web3().get_transaction_count(account).await;
        assert!(response.is_ok());
    }
}