//! Ursa Transport implementation.
//!
//!
//!
use libp2p::{
    core::{
        muxing::StreamMuxerBox,
        transport::{upgrade, Boxed},
        upgrade::SelectUpgrade,
    },
    identity::Keypair,
    mplex, noise,
    relay::v2::client::transport::ClientTransport,
    tcp::GenTcpConfig,
    yamux, PeerId, Transport,
};

use crate::config::NetworkConfig;

pub struct UrsaTransport;

impl UrsaTransport {
    /// Creates a new [`UrsaTransport`].
    ///
    /// Defaults to QUIC transport over TCP.
    /// If QUIC fails to establish a connection, we fail over to TCP.
    pub fn new(
        keypair: &Keypair,
        config: &NetworkConfig,
        relay_transport: Option<ClientTransport>,
    ) -> Boxed<(PeerId, StreamMuxerBox)> {
        let id_keys = keypair;
        let local_peer_id = PeerId::from(keypair.public());

        let tcp = {
            let noise = {
                let dh_keys = noise::Keypair::<noise::X25519Spec>::new()
                    .into_authentic(id_keys)
                    .expect("Signing libp2p-noise static DH keypair failed.");

                noise::NoiseConfig::xx(dh_keys).into_authenticated()
            };

            let mplex = {
                let mut mplex_config = mplex::MplexConfig::new();
                mplex_config.set_max_buffer_behaviour(mplex::MaxBufferBehaviour::Block);
                mplex_config.set_max_buffer_size(usize::MAX);

                let mut yamux_config = yamux::YamuxConfig::default();
                yamux_config.set_window_update_mode(yamux::WindowUpdateMode::on_read());

                SelectUpgrade::new(yamux_config, mplex_config)
            };

            let tcp = libp2p::tcp::TokioTcpTransport::new(GenTcpConfig::default().nodelay(true));

            if let Some(relay) = relay_transport {
                tcp.or_transport(relay)
                    .upgrade(upgrade::Version::V1)
                    .authenticate(noise)
                    .multiplex(mplex)
                    .boxed()
            } else {
                tcp.upgrade(upgrade::Version::V1)
                    .authenticate(noise)
                    .multiplex(mplex)
                    .boxed()
            }
        };
        tcp
    }
}
