use trust_graph::InMemoryStorage;

type TrustGraph = trust_graph::TrustGraph<InMemoryStorage>;

impl Node {
    pub fn new() -> () {}
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
