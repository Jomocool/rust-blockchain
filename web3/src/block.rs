use crate::error::Result;
use crate::Web3;
use ethereum_types::U64;
use jsonrpsee::rpc_params;
use types::block::{Block, BlockNumber};
use types::helpers::to_hex;

impl Web3 {
    /// 将区块号转换为十六进制字符串表示
    ///
    /// 此函数处理区块链中的区块号，将其转换为十六进制字符串格式这对于与区块链节点等外部系统交互时非常有用，
    /// 因为它们通常以十六进制格式接受或返回区块号
    ///
    /// 参数:
    /// - block_number (Option<BlockNumber>): 一个可选的区块号如果未提供区块号（即为None），则函数返回"latest"，
    ///   表示将使用最新的区块信息如果提供了区块号，则将其转换为十六进制字符串表示
    ///
    /// 返回:
    /// - String: 区块号的十六进制字符串表示，或者"latest"如果未提供区块号
    pub(crate) fn get_hex_blocknumber(block_number: Option<BlockNumber>) -> String {
        block_number.map_or_else(
            || "latest".to_string(),
            |block_number| to_hex(*block_number),
        )
    }

    /// 异步获取当前区块链的区块编号
    ///
    /// 该函数通过发送RPC请求`eth_blockNumber`来获取当前区块链的区块编号
    /// 不需要任何参数，返回一个Result类型，其中包含BlockNumber
    ///
    /// # Returns
    ///
    /// - `Result<BlockNumber>`: 返回一个Result类型，包含成功的区块编号或错误信息
    pub async fn get_block_number(&self) -> Result<BlockNumber> {
        // 发送RPC请求以获取当前的区块编号
        let response = self.send_rpc("eth_blockNumber", rpc_params![]).await?;

        // 将RPC响应转换为BlockNumber类型
        let block_number: BlockNumber = serde_json::from_value(response)?;

        // 返回成功的区块编号
        Ok(block_number)
    }

    /// 异步获取指定区块号的区块信息
    ///
    /// 此函数通过以太坊的JSON-RPC接口`eth_getBlockByNumber`请求指定区块号的区块信息
    /// 它首先将区块号转换为十六进制字符串格式，然后构造并发送RPC请求，最后解析响应数据并返回
    ///
    /// # 参数
    ///
    /// * `block_number: U64` - 需要获取信息的区块号，使用U64类型来表示
    ///
    /// # 返回值
    ///
    /// * `Result<Block>` - 返回一个Result类型，包含成功时的Block实例或错误信息
    pub async fn get_block(&self, block_number: U64) -> Result<Block> {
        // 将区块号转换为十六进制字符串格式，以便符合以太坊JSON-RPC的参数要求
        let block_number = to_hex(block_number);
        // 构造RPC请求参数
        let params = rpc_params![block_number];
        // 发送RPC请求并等待响应
        let response = self.send_rpc("eth_getBlockByNumber", params).await?;
        // 解析响应数据为Block类型
        let block: Block = serde_json::from_value(response)?;

        // 返回解析后的区块信息
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
