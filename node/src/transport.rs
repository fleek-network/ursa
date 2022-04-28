//! Fnet Transport implementation.
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
    identity::Keypair,
    mplex, noise,
    tcp::TcpConfig,
    yamux, PeerId, Transport,
};

pub struct FnetTransport {
    keypair: Keypair,
    tcp: TcpConfig,
    quic: TcpConfig,
}

impl FnetTransport {
    /// Creates a new [`FnetTransport`] using keypair.
    pub fn new(keypair: &Keypair) -> Self {
        let id_keys = keypair;

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
                .upgrade(upgrade::Version::V1)
                .authenticate(noise)
                .multiplex(mplex)
                .timeout(Duration::from_secs(20))
                .boxed();

            transport
        };

        let quic = { todo!() };

        FnetTransport {
            keypair: keypair.to_owned(),
            tcp,
            quic,
        }
    }

    /// Builds [`FnetTransport`]
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
