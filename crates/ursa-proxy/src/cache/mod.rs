pub mod moka_cache;

use axum::{async_trait, response::Response};

#[async_trait]
pub trait Cache: Clone + Send + Sync + 'static {
    fn get(&self, key: String) -> Option<Response>;
    fn insert(&self, key: String, value: Vec<u8>);
    fn purge(&self);
}
