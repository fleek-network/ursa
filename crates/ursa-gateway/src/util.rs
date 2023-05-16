use hyper::client::HttpConnector;
use hyper::Body;

pub type Client = hyper::Client<HttpConnector, Body>;
