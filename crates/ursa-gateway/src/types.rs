use hyper::{client::HttpConnector, Body};

pub type Client = hyper::Client<HttpConnector, Body>;
pub type Worker<S, Req> = tower::buffer::Buffer<S, Req>;
