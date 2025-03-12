use ethereum_types::H256;
use jsonrpsee::core::Error;
use jsonrpsee::core::Error as JsonRpseeError;
use jsonrpsee::RpcModule;
use types::{
    account::{Account, AccountData},
    block::BlockNumber,
    helpers::to_hex,
    transaction::TransactionRequest,
};

use crate::{error::Result, server::Context};

/// 在RpcModule中添加一个新的异步方法`eth_add_account`。
///
/// 此函数通过接收一个`RpcModule<Context>`的可变引用来注册一个新的RPC方法，
/// 方法名为"eth_add_account"。当该方法被调用时，它会生成一个随机的账户，
/// 并将其添加到区块链上下文中。
///
/// # 参数
/// * `module`: &mut RpcModule<Context> - RpcModule的可变引用，用于注册RPC方法。
///
/// # 返回值
/// * `Result<()>` - 表示方法注册成功或失败的结果类型。
pub(crate) fn eth_add_account(module: &mut RpcModule<Context>) -> Result<()> {
    // 注册一个名为"eth_add_account"的异步方法到RpcModule中。
    // 该方法不接受任何参数，但需要访问和修改区块链上下文。
    module.register_async_method("eth_addAccount", |_, blockchain| async move {
        // 生成一个随机的账户。
        let key = Account::random();

        // 异步获取区块链上下文的锁，以便添加新账户。
        blockchain
            .lock()
            .await
            .accounts
            // 尝试将新生成的账户添加到区块链上下文中。
            .add_account(&key, &AccountData::new(None))
            // 如果添加失败，将错误转换为JsonRpseeError::Custom。
            .map_err(|e| JsonRpseeError::Custom(e.to_string()))?;

        // 返回新生成的账户公钥作为成功响应。
        Ok(key)
    })?;

    // 函数执行成功，表示方法已成功注册到RpcModule中。
    Ok(())
}

/// 在RpcModule中注册一个异步方法"eth_accounts"
///
/// 该方法允许用户获取当前区块链上下文中所有账户的
/// 它通过异步锁来访问区块链数据结构，并提取账户
///
/// 参数:
/// - module: 一个可变引用到RpcModule，用于注册RPC方法
///
/// 返回:
/// - Result<()>: 表示方法注册成功或失败的空结果类型
pub(crate) fn eth_accounts(module: &mut RpcModule<Context>) -> Result<()> {
    // 注册一个名为"eth_accounts"的异步RPC方法
    module.register_async_method("eth_accounts", |_, blockchain| async move {
        // 异步获取区块链锁，并尝试获取所有账户
        let accounts = blockchain
            .lock()
            .await
            .accounts
            .get_all_accounts()
            // 如果获取账户信息时发生错误，将其转换为JsonRpseeError::Custom
            .map_err(|e| JsonRpseeError::Custom(e.to_string()))?;

        // 成功获取账户信息后，返回账户
        Ok(accounts)
    })?;

    // 函数执行成功，返回Ok(())
    Ok(())
}

/// 在RpcModule中注册一个异步方法，用于获取当前区块链的块号。
///
/// # 参数
///
/// * `module`: 一个可变引用到RpcModule，用于注册RPC方法。
///
/// # 返回值
///
/// 返回一个Result，表示方法注册成功与否。
pub(crate) fn eth_block_number(module: &mut RpcModule<Context>) -> Result<()> {
    // 注册一个名为"eth_blockNumber"的异步RPC方法。
    module.register_async_method("eth_blockNumber", |_, blockchain| async move {
        // 异步获取区块链锁，并尝试获取当前块的信息。
        let block_number = blockchain
            .lock()
            .await
            .get_current_block()
            // 如果获取块信息时发生错误，将其转换为JsonRpseeError::Custom错误返回。
            .map_err(|e| JsonRpseeError::Custom(e.to_string()))?
            .number;
        // 返回当前块的编号。
        Ok(block_number)
    })?;

    // 方法注册成功，返回Ok。
    Ok(())
}

/// 在RpcModule中注册一个异步方法，用于根据区块编号获取区块信息。
///
/// 此函数通过引用可变的RpcModule<Context>实例来注册一个名为"eth_getBlockByNumber"的异步方法。
/// 该方法允许客户端通过RPC调用请求特定编号的区块信息。
///
/// # 参数
/// * `module`: &mut RpcModule<Context> - 一个可变引用，指向RpcModule实例，用于注册RPC方法。
///
/// # 返回值
/// * `Result<()>` - 表示方法注册操作的成功或失败，成功时返回()。
pub(crate) fn eth_get_block_by_number(module: &mut RpcModule<Context>) -> Result<()> {
    // 注册一个名为"eth_getBlockByNumber"的异步方法到RpcModule中。
    // 该方法接收两个参数：params（包含方法参数）和blockchain（一个异步锁，用于访问区块链数据）。
    // 并返回一个异步结果，该结果在方法解析时产生。
    module.register_async_method("eth_getBlockByNumber", |params, blockchain| async move {
        // 从参数中提取BlockNumber，这可能是一个具体的区块编号或最新的区块标识。
        let block_number = params.one::<BlockNumber>()?;
        // 锁定区块链数据结构以获取指定编号的区块信息。
        // 这里使用了异步锁来防止阻塞线程，并调用get_block_by_number方法获取区块。
        let block = blockchain.lock().await.get_block_by_number(*block_number)?;

        // 返回获取的区块信息作为RPC调用的结果。
        Ok(block)
    })?;

    // 函数执行成功，返回Ok(())表示方法注册成功。
    Ok(())
}

/// 在RpcModule中注册一个异步方法`eth_getBalance`来获取账户余额
///
/// # Parameters
///
/// * `module`: 一个可变引用到`RpcModule<Context>`，用于注册RPC方法
///
/// # Returns
///
/// * `Result<()>`: 一个结果类型，表示方法注册成功或失败
///
/// # Remarks
///
/// 该函数将`eth_getBalance`方法注册到RPC模块中，当该方法被调用时，它会解析请求参数，
/// 从区块链中获取当前区块号，并检索指定账户的余额，最后将余额转换为十六进制字符串返回
pub(crate) fn eth_get_balance(module: &mut RpcModule<Context>) -> Result<()> {
    // 注册一个异步RPC方法`eth_getBalance`
    module.register_async_method("eth_getBalance", move |params, blockchain| async move {
        // 从请求参数中解析出账户信息
        let key = params.one::<Account>()?;

        // 根据账户信息获取账户余额
        let balance = blockchain
            .lock()
            .await
            .accounts
            .get_account(&key)
            .map_err(|e| Error::Custom(e.to_string()))?
            .balance;

        // 将账户余额转换为十六进制字符串并返回
        Ok(to_hex(balance))
    })?;

    Ok(())
}

// 在RpcModule中注册一个异步方法，用于获取账户的交易计数
pub(crate) fn eth_get_transaction_count(module: &mut RpcModule<Context>) -> Result<()> {
    // 注册一个名为"eth_getTransactionCount"的异步方法
    module.register_async_method("eth_getTransactionCount", |params, blockchain| async move {
        // 从参数中解析出账户信息
        let account = params.one::<Account>()?;
        // 获取账户的交易计数
        let count = blockchain
            .lock()
            .await
            .accounts
            .get_account(&account)
            .map_err(|e| Error::Custom(e.to_string()))?
            .nonce;

        // 将交易计数转换为十六进制字符串并返回
        Ok(to_hex(count))
    })?;

    // 表示方法注册成功
    Ok(())
}

/// 在RpcModule中注册一个异步方法用于发送交易
///
/// 该函数向RpcModule<Context>类型的一个实例中注册了一个名为"eth_sendTransaction"的异步方法
/// 当该方法被调用时，它会解析传入的参数以构建一个交易请求，然后在区块链上发送该交易
/// 主要解决了如何通过RPC接口发送交易的问题
///
/// # Parameters
///
/// * `module`: &mut RpcModule<Context> - 一个可变引用，指向RpcModule实例，用于注册RPC方法
///
/// # Returns
///
/// * `Result<()>` - 表示方法注册成功或失败的结果，成功时返回空元组
pub(crate) fn eth_send_transaction(module: &mut RpcModule<Context>) -> Result<()> {
    // 注册一个名为"eth_sendTransaction"的异步方法
    // 该方法接收一个参数和一个Blockchain的引用，返回一个异步结果
    module.register_async_method(
        "eth_sendTransaction",
        move |params, blockchain| async move {
            // 从参数中解析出一个TransactionRequest实例
            let transaction_request = params.one::<TransactionRequest>()?;
            // 获取Blockchain的锁，以确保线程安全，然后发送交易
            let transaction_hash = blockchain
                .lock()
                .await
                .send_transaction(transaction_request)
                .await;

            // 返回发送交易后的哈希值
            Ok(transaction_hash?)
        },
    )?;

    Ok(())
}

// 在RpcModule中注册一个异步方法，用于获取交易收据
pub(crate) fn eth_get_transaction_receipt(module: &mut RpcModule<Context>) -> Result<()> {
    // 注册一个名为"eth_getTransactionReceipt"的异步方法
    module.register_async_method(
        "eth_getTransactionReceipt",
        // 使用闭包定义方法的逻辑
        move |params, blockchain| async move {
            // 从参数中提取交易哈希
            let transaction_hash = params.one::<H256>()?;
            // 获取区块链锁，并尝试获取交易收据
            let transaction_receipt = blockchain
                .lock()
                .await
                .get_transaction_receipt(transaction_hash)
                .await
                // 如果获取失败，返回自定义错误
                .map_err(|e| Error::Custom(e.to_string()))?;

            // 返回获取到的交易收据
            Ok(transaction_receipt)
        },
    )?;

    // 方法注册成功，返回Ok
    Ok(())
}

// 在RpcModule中注册以太坊获取智能合约代码的异步方法
// 该函数负责处理来自RPC的请求，获取指定地址和区块的代码哈希
pub(crate) fn eth_get_code(module: &mut RpcModule<Context>) -> Result<()> {
    // 注册一个名为"eth_getCode"的异步方法
    // 该方法接受两个参数：params（请求参数）和blockchain（区块链数据）
    module.register_async_method("eth_getCode", move |params, blockchain| async move {
        // 创建一个序列对象，用于解析传入的参数
        let mut seq = params.sequence();
        // 解析第一个参数：账户地址
        let address = seq.next::<Account>()?;

        // 获取指定合约账户的代码哈希
        let code_hash = blockchain
            .lock()
            .await
            .accounts
            .get_account(&address)
            .map_err(|e| Error::Custom(e.to_string()))?
            .code_hash
            .ok_or_else(|| {
                JsonRpseeError::Custom(format!("missing code hash for account {:?}", address))
            })?;

        // 返回代码哈希
        Ok(code_hash)
    })?;

    // 表示函数执行成功
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::helpers::tests::setup;

    #[tokio::test]
    async fn gets_an_account_balance() {
        let (blockchain, id_1, _) = setup().await;
        let balance = blockchain
            .lock()
            .await
            .accounts
            .get_account(&id_1)
            .unwrap()
            .balance;
        let mut module = RpcModule::new(blockchain);
        eth_get_balance(&mut module).unwrap();
        let response: String = module.call("eth_getBalance", [id_1]).await.unwrap();

        assert_eq!(response, to_hex(balance));
    }
}
