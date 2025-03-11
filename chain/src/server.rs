use hyper::Method;
use jsonrpsee::{
    server::{ServerBuilder, ServerHandle},
    RpcModule,
};
use std::{env, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{sync::Mutex, task, time};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{util::SubscriberInitExt, FmtSubscriber};

use crate::{
    blockchain::BlockChain,
    error::{ChainError, Result},
    keys::{add_keys, ADDRESS},
    logger::Logger,
    method::*,
};

pub(crate) type Context = Arc<Mutex<BlockChain>>;

pub(crate) async fn serve(addr: &str, blockchain: Context) -> Result<ServerHandle> {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }

    FmtSubscriber::builder().finish().try_init()?;

    add_keys()?;

    let addrs = addr.parse::<SocketAddr>()?;
    let cors = CorsLayer::new()
        .allow_methods([Method::POST])
        .allow_origin(Any)
        .allow_headers([hyper::header::CONTENT_TYPE]);
    let middleware = tower::ServiceBuilder::new().layer(cors);
    let server = ServerBuilder::default()
        .set_logger(Logger)
        .set_middleware(middleware)
        .build(addrs)
        .await?;
    let blockchain_for_transaction_processor = blockchain.clone();
    let mut module = RpcModule::new(blockchain);

    eth_add_account(&mut module)?;
    eth_accounts(&mut module)?;
    eth_block_number(&mut module)?;
    eth_get_block_by_number(&mut module)?;
    eth_get_balance(&mut module)?;
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

    let transaction_processor = task::spawn(async move {
        let mut interval = time::interval(Duration::from_millis(1000));

        // 循环不断处理交易池中的交易
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
