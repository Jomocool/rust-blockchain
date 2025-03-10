use crate::error::Result;
use crate::Web3;
use ethereum_types::H256;
use jsonrpsee::rpc_params;
use serde_json::to_value;
use types::bytes::Bytes;
use types::transaction::{TransactionReceipt, TransactionRequest};

impl Web3 {
    /// 异步发送交易请求
    ///
    /// 该函数接受一个TransactionRequest对象作为参数，将其转换为JSON-RPC参数格式，并调用以太坊的eth_sendTransaction方法
    /// 发送交易。成功后，返回交易的哈希值
    ///
    /// 参数:
    /// - transaction_request: TransactionRequest类型，包含交易必要信息的请求对象
    ///
    /// 返回:
    /// - Result类型，包含交易的哈希值（H256）。如果发送交易过程中出现错误，则返回一个错误
    pub async fn send(&self, transaction_request: TransactionRequest) -> Result<H256> {
        // 将TransactionRequest对象转换为Serde JSON值
        let transaction_request = to_value(&transaction_request)?;
    
        // 构造JSON-RPC参数
        let params = rpc_params![transaction_request];
    
        // 发送JSON-RPC请求并等待响应
        let response = self.send_rpc("eth_sendTransaction", params).await?;
    
        // 从响应中解析出交易哈希值
        let tx_hash: H256 = serde_json::from_value(response)?;
    
        // 返回交易哈希值
        Ok(tx_hash)
    }

    /// 异步发送原始交易请求到以太坊节点
    ///
    /// 该函数接收一个包含交易数据的字节对象，通过RPC调用发送交易到以太坊网络，
    /// 并返回交易的哈希值
    ///
    /// 参数:
    /// - `transaction_request`: 包含交易数据的字节对象
    ///
    /// 返回:
    /// - `Result<H256>`: 一个包含交易哈希的结果对象如果发送成功，否则包含一个错误
    pub async fn send_raw(&self, transaction_request: Bytes) -> Result<H256> {
        // 将交易请求数据序列化为JSON值
        let transaction_request = to_value(&transaction_request)?;
        // 构造RPC调用参数
        let params = rpc_params![transaction_request];
        // 发送RPC调用并等待响应
        let response = self.send_rpc("eth_sendRawTransaction", params).await?;
        // 从响应中反序列化出交易哈希值
        let tx_hash: H256 = serde_json::from_value(response)?;
    
        // 返回交易哈希值
        Ok(tx_hash)
    }

    /// 异步获取交易收据
    ///
    /// 本函数通过RPC调用以太坊节点获取指定交易哈希的交易收据
    /// 主要用于查询交易的详细信息，如 gas 使用情况、日志等
    ///
    /// # 参数
    /// * `tx_hash` - 交易哈希，类型为H256，用于唯一标识一笔交易
    ///
    /// # 返回值
    /// 返回一个 `Result` 类型，包含 `TransactionReceipt` 对象
    /// 如果获取收据成功，则返回 Ok(receipt)；如果发生错误，则返回 Err(error)
    ///
    /// # 错误处理
    /// * 如果 `to_value` 转换失败，会返回一个错误
    /// * 如果 `send_rpc` 调用失败，会返回一个错误
    /// * 如果 `serde_json::from_value` 解析失败，会返回一个错误
    pub async fn transaction_receipt(&self, tx_hash: H256) -> Result<TransactionReceipt> {
        // 将交易哈希转换为 RPC 调用所需的值类型
        let tx_hash = to_value(tx_hash)?;
        // 构造 RPC 调用参数
        let params = rpc_params![tx_hash];
        // 发送 RPC 调用并等待响应
        let response = self.send_rpc("eth_getTransactionReceipt", params).await?;
        // 解析响应数据为 TransactionReceipt 类型
        let receipt = serde_json::from_value(response)?;
    
        // 返回解析后的交易收据
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
