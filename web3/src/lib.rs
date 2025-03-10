use crate::error::{Result, Web3Error};
use jsonrpsee::core::client::ClientT;
use jsonrpsee::core::traits::ToRpcParams;
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use log::*;
use serde_json::Value;

pub mod account;
pub mod block;
pub mod contract;
pub mod error;
mod helpers;
pub mod transaction;

pub struct Web3 {
    client: HttpClient,
}

impl Web3 {
    pub fn new(url: &str) -> Result<Self> {
        let client = Web3::get_client(url)?;
        Ok(Self { client })
    }

    fn get_client(url: &str) -> Result<HttpClient> {
        HttpClientBuilder::default()
            .build(url)
            .map_err(|e| Web3Error::ClientError(e.to_string()))
    }

    pub async fn send_rpc<Params>(&self, method: &str, params: Params) -> Result<Value>
    where
        Params: ToRpcParams + Send + std::fmt::Debug,
    {
        trace!("Sending RPC {} with params {:?}", method, params);

        let response = self
            .client
            .request(method, params)
            .await
            .map_err(|e| Web3Error::RpcRequestError(e.to_string()));

        trace!("RPC Response {:?}", response);

        response
    }
}