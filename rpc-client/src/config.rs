use std::net::{Ipv4Addr, SocketAddrV4};

#[derive(Debug)]
pub struct Config {
    /// Listen address
    pub listen: SocketAddrV4,
    /// Swarm listening Address.
    pub swarm_addr: SocketAddrV4,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080),
            swarm_addr: SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080),
        }
    }
}
