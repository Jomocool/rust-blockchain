//! # Contracts
//!
//! Deploy and interact with contracts on Ethereum.

use crate::error::Result;
use crate::Web3;
use ethereum_types::Address;
use ethereum_types::{H256, U256};
use jsonrpsee::rpc_params;
use types::block::BlockNumber;
use types::bytes::Bytes;
use types::helpers::to_hex;
use types::transaction::TransactionRequest;

impl Web3 {
    /// Deploy a contract to the chain.
    pub async fn deploy<'a>(
        &self,
        owner: Address,
        abi: &'a [u8],
        nonce: Option<U256>,
    ) -> Result<H256> {
        let gas = U256::from(1_000_000);
        let gas_price = U256::from(1_000_000);
        let data: Bytes = abi.to_vec().into();
        let transaction_request = TransactionRequest {
            from: Some(owner),
            to: None,
            value: Some(U256::zero()),
            gas,
            gas_price,
            data: Some(data),
            nonce,
            r: None,
            s: None,
        };

        self.send(transaction_request).await
    }

    /// Get the contract code for an address
    pub async fn code(
        &self,
        address: Address,
        block_number: Option<BlockNumber>,
    ) -> Result<Vec<u8>> {
        let block_number = Web3::get_hex_blocknumber(block_number);
        let params = rpc_params![to_hex(address), block_number];
        let response = self.send_rpc("eth_getCode", params).await?;
        let code: Vec<u8> = serde_json::from_value(response)?;

        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use crate::helpers::tests::{deploy_contract, web3};
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn it_deploys_a_contract() {
        let response = deploy_contract(true).await;
    }

    #[tokio::test]
    async fn it_gets_a_contract_code() {
        let web3 = web3();
        let tx_hash = deploy_contract(true).await;

        // TODO(ddimaria): use polling or callbacks instead of waiting
        sleep(Duration::from_millis(1000)).await;

        let receipt = web3.transaction_receipt(tx_hash).await.unwrap();
        let response = web3.code(receipt.contract_address.unwrap(), None).await;

        // ensure the code matches what was deployed
        assert_eq!(response.unwrap(), [0, 1]);
    }
}
