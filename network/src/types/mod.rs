use libp2p::request_response::RequestResponseEvent;

use crate::codec::proto::{UrsaExchangeRequest, UrsaExchangeResponse};

pub type UrsaRequestResponseEvent = RequestResponseEvent<UrsaExchangeRequest, UrsaExchangeResponse>;
