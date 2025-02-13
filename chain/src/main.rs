//! # Web3

mod account;
mod blockchain;
mod error;
mod helpers;
mod keys;
mod logger;
mod method;
mod server;
mod storage;
mod transaction;
mod world_state;

use error::Result;
use server::serve;

#[tokio::main]
async fn main() -> Result<()> {
    let (blockchain, _, _) = crate::helpers::tests::setup().await;
    let _server = serve("127.0.0.1:8545", blockchain).await?;

    // create a future that never resolves
    futures::future::pending().await
}
