use async_trait::async_trait;
use futures::AsyncRead;
use libp2p::{core::ProtocolName, request_response::RequestResponseCodec};

pub const PROTOCOL_NAME: &[u8] = b"/ursa/txrx/1.0.0";

#[derive(Debug, Clone)]
pub struct UrsaExchangeProtocol;

impl ProtocolName for UrsaExchangeProtocol {
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
    type Protocol = UrsaExchangeProtocol;

    type Request = UrsaExchangeRequest;

    type Response = UrsaExchangeResponse;

    fn read_request<T>(
        &mut self,
        protocol: &Self::Protocol,
        io: &mut T,
    ) -> std::io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        todo!()
    }

    fn read_response<T>(
        &mut self,
        protocol: &Self::Protocol,
        io: &mut T,
    ) -> std::io::Result<Self::Response>
    where
        T: futures::AsyncRead + Unpin + Send,
    {
        todo!()
    }

    fn write_request<T>(
        &mut self,
        protocol: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> std::io::Result<()>
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
    ) -> std::io::Result<()>
    where
        T: futures::AsyncWrite + Unpin + Send,
    {
        todo!()
    }
}
