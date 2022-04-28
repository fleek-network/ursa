mod behaviour;
mod config;
mod discovery;
mod gossip;
mod rpc;
mod service;
mod transport;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
