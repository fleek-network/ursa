mod core;

#[tokio::main]
async fn main() {
    core::start_server().await.unwrap()
}
