use hyper::client::HttpConnector;
use hyper::Body;

pub mod error;

pub type Client = hyper::Client<HttpConnector, Body>;
