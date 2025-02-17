//! # Server
//!
//! Start the JsonRPC server and register methods

use jsonrpsee::{
    server::{ServerBuilder, ServerHandle},
    RpcModule,
};
use std::{env, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{sync::Mutex, task, time};
use tracing_subscriber::{util::SubscriberInitExt, FmtSubscriber};

use crate::{
    blockchain::BlockChain,
    error::{ChainError, Result},
    keys::{add_keys, ADDRESS},
    logger::Logger,
    method::*,
};

pub(crate) type Context = Arc<Mutex<BlockChain>>;

// jsonrpsee requires static lifetimes for state
pub(crate) async fn serve(addr: &str, blockchain: Context) -> Result<ServerHandle> {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }

    FmtSubscriber::builder().finish().try_init()?;

    // generate keys if necessary
    add_keys()?;

    let addrs = addr.parse::<SocketAddr>()?;
    let server = ServerBuilder::default()
        .set_logger(Logger)
        .build(addrs)
        .await?;
    let blockchain_for_transaction_processor = blockchain.clone();
    let mut module = RpcModule::new(blockchain);

    // register methods
    eth_block_number(&mut module)?;
    eth_get_block_by_number(&mut module)?;
    eth_get_balance(&mut module)?;
    eth_get_balance_by_block(&mut module)?;
    eth_send_transaction(&mut module)?;
    eth_send_raw_transaction(&mut module)?;
    eth_get_transaction_receipt(&mut module)?;
    eth_get_transaction_count(&mut module)?;
    eth_get_code(&mut module)?;

    let server_handle = server.start(module)?;

    tracing::info!(
        "Starting server on {}, with public address {:?}",
        addrs,
        *ADDRESS
    );

    // process transactions in a separate thread
    let transaction_processor = task::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(1000));

        loop {
            interval.tick().await;

            if let Err(error) = blockchain_for_transaction_processor
                .lock()
                .await
                .process_transactions()
                .await
            {
                tracing::error!("Error processing transactions {}", error.to_string());
            }
        }
    });

    transaction_processor
        .await
        .map_err(|e| ChainError::InternalError(e.to_string()))?;

    Ok(server_handle)
}
