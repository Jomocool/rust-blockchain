use crate::error::{Result, Web3Error};
use crate::Web3;
use ethereum_types::U256;
use jsonrpsee::rpc_params;
use types::account::Account;
use types::helpers::to_hex;
use types::transaction::{SignedTransaction, Transaction};
use utils::crypto::SecretKey;

impl Web3 {
    /// 获取指定地址的余额。
    pub async fn get_balance(&self, address: Account) -> Result<U256> {
        let params = rpc_params![to_hex(address)];
        let response = self.send_rpc("eth_getBalance", params).await?;
        let balance: U256 = serde_json::from_value(response)?;

        Ok(balance)
    }

    /// 签名交易。
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

    /// 获取账户的交易数量
    pub async fn get_transaction_count(&self, address: Account) -> Result<U256> {
        let params = rpc_params![to_hex(address)];
        let response = self.send_rpc("eth_getTransactionCount", params).await?;
        let balance: U256 = serde_json::from_value(response)?;

        Ok(balance)
    }
}
