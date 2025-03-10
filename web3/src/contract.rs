
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
    // 部署智能合约的异步函数
    // 
    // 该函数负责将编译后的智能合约代码（ABI）部署到区块链网络上。它需要合约的拥有者地址、
    // 合约的字节码（ABI）、以及一个可选的交易nonce值。函数会构建一个交易请求，并发送到
    // 区块链网络，等待部署成功并返回交易的哈希值。
    //
    // 参数:
    // - owner: 合约拥有者的地址，用于标识部署合约的账户
    // - abi: 智能合约的字节码，以字节流形式提供
    // - nonce: 可选的交易计数器，用于指定交易的顺序
    //
    // 返回值:
    // - Result<H256>: 如果部署成功，返回交易的哈希值；如果失败，返回错误
    pub async fn deploy<'a>(
        &self,
        owner: Address,
        abi: &'a [u8],
        nonce: Option<U256>,
    ) -> Result<H256> {
        // 设置交易的基本参数
        let gas = U256::from(1_000_000); // 设置Gas限制，用于限制交易执行所消耗的最大Gas量
        let gas_price = U256::from(1_000_000); // 设置Gas价格，用于指定每单位Gas的价格
        let data: Bytes = abi.to_vec().into(); // 将ABI字节码转换为交易数据
    
        // 构建交易请求对象，包含所有必要的交易信息
        let transaction_request = TransactionRequest {
            from: Some(owner), // 指定交易的发送者地址
            to: None, // 交易的目标地址，对于合约部署来说是None
            value: Some(U256::zero()), // 交易附带的以太币价值，这里设置为0
            gas,
            gas_price,
            data: Some(data), // 交易数据，包含合约的字节码
            nonce, // 交易的nonce值，用于保证交易顺序
            r: None, // 交易的r签名值，此处不需要提供
            s: None, // 交易的s签名值，此处不需要提供
        };
    
        // 发送构建好的交易请求，并等待结果
        self.send(transaction_request).await
    }

    /// 异步获取指定地址和区块号的代码信息
    ///
    /// 此函数通过发送RPC请求来获取智能合约的字节码信息它接受一个必需的地址参数和一个可选的区块号参数
    /// 如果区块号未指定，将使用默认的最新区块号
    ///
    /// # 参数
    ///
    /// * `address` - 合约地址，必须为有效的Address类型
    /// * `block_number` - 可选的区块号，用于指定从哪个区块获取代码信息如果未提供，则使用最新区块
    ///
    /// # 返回值
    ///
    /// 返回一个Result类型，包含字节码信息（Vec<u8>）如果请求成功，字节码信息将被解析并返回；
    /// 如果请求失败或解析错误，将返回一个错误
    pub async fn code(
        &self,
        address: Address,
        block_number: Option<BlockNumber>,
    ) -> Result<Vec<u8>> {
        // 将区块号转换为十六进制字符串，以便符合以太坊RPC的参数要求
        let block_number = Web3::get_hex_blocknumber(block_number);
        // 构建RPC请求参数数组，包含地址和区块号
        let params = rpc_params![to_hex(address), block_number];
        // 发送RPC请求并等待响应
        let response = self.send_rpc("eth_getCode", params).await?;
        // 从响应中解析字节码信息
        let code: Vec<u8> = serde_json::from_value(response)?;
    
        // 返回解析后的字节码信息
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

        sleep(Duration::from_millis(1000)).await;

        let receipt = web3.transaction_receipt(tx_hash).await.unwrap();
        let response = web3.code(receipt.contract_address.unwrap(), None).await;

        assert_eq!(response.unwrap(), [0, 1]);
    }
}
