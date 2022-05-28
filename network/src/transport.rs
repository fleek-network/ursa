//! Ursa Transport implementation.
//!
//!
//!

use std::time::Duration;

use async_std::task::block_on;
use libp2p::{
    core::{
        either::EitherOutput,
        muxing::StreamMuxerBox,
        transport::{upgrade, Boxed, OrTransport},
        upgrade::SelectUpgrade,
    },
    dns::DnsConfig,
    mplex, noise,
    relay::v2::client::Client as RelayClient,
    tcp::TcpConfig,
    yamux, PeerId, Transport,
};

use crate::config::UrsaConfig;

pub struct UrsaTransport {
    tcp: TcpConfig,
    quic: TcpConfig,
    relay_client: RelayClient,
}

impl UrsaTransport {
    /// Creates a new [`UrsaTransport`] using keypair.
    pub fn new(config: &UrsaConfig) -> Self {
        let id_keys = config.keypair;
        let local_peer_id = PeerId::from(config.keypair.public());

        let (relay_transport, relay_client) = if config.relay {
            RelayClient::new_transport_and_behaviour(local_peer_id)
        } else {
            None
        };

        let tcp = {
            let noise = {
                let dh_keys = noise::Keypair::<noise::X25519Spec>::new()
                    .into_authentic(&id_keys)
                    .expect("Signing libp2p-noise static DH keypair failed.");

                noise::NoiseConfig::xx(dh_keys).into_authenticated()
            };

            let mplex = {
                SelectUpgrade::new(yamux::YamuxConfig::default(), mplex::MplexConfig::default())
            };

            let transport = block_on(DnsConfig::system(
                TcpConfig::new().nodelay(true).port_reuse(true),
            ))
            .unwrap();

            transport
                .or_transport(relay_transport)
                .upgrade(upgrade::Version::V1)
                .authenticate(noise)
                .multiplex(mplex)
                .timeout(Duration::from_secs(20))
                .boxed();

            transport
        };

        let quic = {
            // block_on(QuicTransport::new(
            //     QuicConfig::new(keypair),
            //     quic_addr.unwrap_or("/ip4/0.0.0.0/udp/0/quic".parse().unwrap()),
            // ))
            // .unwrap()
            todo!()
        };

        UrsaTransport {
            tcp,
            quic,
            relay_client,
        }
    }

    /// Builds [`UrsaTransport`]
    ///
    /// Defaults to QUIC transport over TCP.
    /// If QUIC fails to establish a connection, we failover to TCP.
    pub fn build(&self) -> Boxed<(PeerId, StreamMuxerBox)> {
        // self.quic.or_transport(self.tcp)
        // self.tcp.or_transport(self.quic).boxed()
        OrTransport::new(self.quic, self.tcp)
            .map(|either_output, _| match either_output {
                EitherOutput::First((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
                EitherOutput::Second((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
            })
            .boxed()
    }
}
