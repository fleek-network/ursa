//! Ursa Transport implementation.
//!
//!
//!

use libp2p::core::transport::{upgrade, Boxed};
use libp2p::tcp::GenTcpConfig;
use libp2p::Transport;
use libp2p::{
    core::{either::EitherOutput, muxing::StreamMuxerBox, upgrade::SelectUpgrade},
    identity::Keypair,
    mplex, noise,
    relay::v2::client::Client as RelayClient,
    tcp, yamux, PeerId,
};

use crate::config::UrsaConfig;

pub struct UrsaTransport;

impl UrsaTransport {
    /// Creates a new [`UrsaTransport`].
    ///
    /// Defaults to QUIC transport over TCP.
    /// If QUIC fails to establish a connection, we fail over to TCP.
    pub fn new(keypair: &Keypair, config: &UrsaConfig) -> Boxed<(PeerId, StreamMuxerBox)> {
        let id_keys = keypair;
        let local_peer_id = PeerId::from(keypair.public());

        // let relay = if config.relay {
        //     Some(RelayClient::new_transport_and_behaviour(local_peer_id))
        // } else {
        //     None
        // };

        let tcp = {
            let noise = {
                let dh_keys = noise::Keypair::<noise::X25519Spec>::new()
                    .into_authentic(id_keys)
                    .expect("Signing libp2p-noise static DH keypair failed.");

                noise::NoiseConfig::xx(dh_keys).into_authenticated()
            };

            let mplex = {
                SelectUpgrade::new(yamux::YamuxConfig::default(), mplex::MplexConfig::default())
            };

            let tcp = libp2p::tcp::TokioTcpTransport::new(GenTcpConfig::default().nodelay(true));

            tcp.upgrade(upgrade::Version::V1)
                .authenticate(noise)
                .multiplex(mplex)
                .boxed()
        };

        // let quic = {
        //     // block_on(QuicTransport::new(
        //     //     QuicConfig::new(keypair),
        //     //     quic_addr.unwrap_or("/ip4/0.0.0.0/udp/0/quic".parse().unwrap()),
        //     // ))
        //     // .unwrap()
        //     todo!()
        // };
        // self.quic.or_transport(self.tcp)

        // OrTransport::new(tcp, tcp)
        //     .map(|either_output, _| match either_output {
        //         EitherOutput::First((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
        //         EitherOutput::Second((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
        //     })
        //     .boxed()
        tcp
    }
}
