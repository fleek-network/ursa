use std::pin::Pin;

use anyhow::Result;
use async_trait::async_trait;
use futures::{AsyncRead, Future};
use libp2p::{core::ProtocolName, request_response::RequestResponseCodec};

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

#[derive(Debug, Clone, PartialEq)]
pub struct UrsaExchangeRequest;

#[derive(Debug, Clone, PartialEq)]
pub struct UrsaExchangeResponse;

#[async_trait]
impl RequestResponseCodec for UrsaExchangeCodec {
    type Protocol = UrsaProtocol;

    type Request = UrsaExchangeRequest;

    type Response = UrsaExchangeResponse;

    fn read_request<T>(
        &mut self,
        protocol: &Self::Protocol,
        io: &mut T,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Request>> + Send>>
    where
        T: AsyncRead + Unpin + Send,
    {
        todo!()
    }

    fn read_response<T>(
        &mut self,
        protocol: &Self::Protocol,
        io: &mut T,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Response>> + Send>>
    where
        T: AsyncRead + Unpin + Send,
    {
        todo!()
    }

    fn write_request<T>(
        &mut self,
        protocol: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>
    where
        T: futures::AsyncWrite + Unpin + Send,
    {
        todo!()
    }

    fn write_response<T>(
        &mut self,
        protocol: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>
    where
        T: futures::AsyncWrite + Unpin + Send,
    {
        todo!()
    }
}
