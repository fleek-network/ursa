use hyper::{client::HttpConnector, Body};
use hyper_tls::HttpsConnector;

pub mod get;
pub mod put;

type Client = hyper::client::Client<HttpsConnector<HttpConnector>, Body>;
