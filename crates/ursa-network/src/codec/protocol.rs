use crate::utils::cache_summary::CacheSummary;
use async_trait::async_trait;
use futures::{AsyncRead, AsyncWrite, AsyncWriteExt};
use libipld::Cid;
use libp2p::{
    core::{
        upgrade::{read_length_prefixed, write_length_prefixed},
        ProtocolName,
    },
    request_response::RequestResponseCodec,
};
use serde::{Deserialize, Serialize};
use std::io;

/// Max request size in bytes
const MAX_REQUEST_SIZE: usize = 4 * 1024 * 1024; // 1 << 22
/// Max response size in bytes
const MAX_RESPONSE_SIZE: usize = 10 * 1024 * 1024;

pub const PROTOCOL_NAME: &[u8] = b"/ursa/txrx/0.0.1";

#[derive(Debug, Clone)]
pub struct UrsaProtocol;

impl ProtocolName for UrsaProtocol {
    fn protocol_name(&self) -> &[u8] {
        PROTOCOL_NAME
    }
}

#[derive(Debug, Clone)]
pub struct UrsaExchangeCodec;

// todo(botch): think of a proper structure for a request
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestType {
    // change this to the final cid version
    CarRequest(String),
    CacheRequest(Cid),
    StoreSummary(Box<CacheSummary>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrsaExchangeRequest(pub RequestType);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CarResponse {
    // change this to the final cid version
    pub cid: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResponseType {
    CarResponse(CarResponse),
    CacheResponse,
    StoreSummaryRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrsaExchangeResponse(pub ResponseType);

#[async_trait]
impl RequestResponseCodec for UrsaExchangeCodec {
    type Protocol = UrsaProtocol;

    type Request = UrsaExchangeRequest;

    type Response = UrsaExchangeResponse;

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let vec = read_length_prefixed(io, MAX_REQUEST_SIZE).await?;

        if vec.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        let request: UrsaExchangeRequest =
            serde_json::from_str(&String::from_utf8(vec).unwrap()).unwrap();

        Ok(request)
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let vec = read_length_prefixed(io, MAX_RESPONSE_SIZE).await?;

        if vec.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        let response: UrsaExchangeResponse =
            serde_json::from_str(&String::from_utf8(vec).unwrap()).unwrap();

        Ok(response)
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&req).unwrap();
        write_length_prefixed(io, &data).await?;
        io.close().await?;

        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let data = serde_json::to_vec(&res).unwrap();
        write_length_prefixed(io, &data).await?;
        io.close().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[ignore = "todo"]
    #[tokio::test]
    async fn test_read_request() {
        todo!()
    }

    #[ignore = "todo"]
    #[tokio::test]
    async fn test_read_response() {
        todo!()
    }

    #[ignore = "todo"]
    #[tokio::test]
    async fn test_write_request() {
        todo!()
    }

    #[ignore = "todo"]
    #[tokio::test]
    async fn test_write_response() {
        todo!()
    }
}
