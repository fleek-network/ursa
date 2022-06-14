use std::{io, pin::Pin};

use anyhow::Result;
use async_trait::async_trait;
use futures::{AsyncRead, AsyncWrite};
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

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        todo!()
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        todo!()
    }

    async fn write_request<T>(
        &mut self,
        protocol: &Self::Protocol,
        _: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        todo!()
    }

    async fn write_response<T>(
        &mut self,
        protocol: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: futures::AsyncWrite + Unpin + Send,
    {
        todo!()
    }
}
