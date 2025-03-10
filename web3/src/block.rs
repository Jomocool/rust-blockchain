use crate::error::Result;
use crate::Web3;
use ethereum_types::U64;
use jsonrpsee::rpc_params;
use types::block::{Block, BlockNumber};
use types::helpers::to_hex;

impl Web3 {
    pub(crate) fn get_hex_blocknumber(block_number: Option<BlockNumber>) -> String {
        block_number.map_or_else(
            || "latest".to_string(),
            |block_number| to_hex(*block_number),
        )
    }

    pub async fn get_block_number(&self) -> Result<BlockNumber> {
        let response = self.send_rpc("eth_blockNumber", rpc_params![]).await?;
        let block_number: BlockNumber = serde_json::from_value(response)?;

        Ok(block_number)
    }

    pub async fn get_block(&self, block_number: U64) -> Result<Block> {
        let block_number = to_hex(block_number);
        let params = rpc_params![block_number];
        let response = self.send_rpc("eth_getBlockByNumber", params).await?;
        let block: Block = serde_json::from_value(response)?;

        Ok(block)
    }
}

#[cfg(test)]
mod tests {
    use crate::helpers::tests::web3;

    #[tokio::test]
    async fn it_gets_a_block_number() {
        let response = web3().get_block_number().await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn it_gets_the_latest_block() {
        let block_number = web3().get_block_number().await.unwrap();
        let response = web3().get_block(*block_number).await;
        assert!(response.is_ok());
    }
}
