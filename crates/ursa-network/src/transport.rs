//! Ursa Transport implementation.
use libp2p::{
    core::{
        muxing::StreamMuxerBox,
        transport::{upgrade, Boxed, OrTransport},
        upgrade::SelectUpgrade,
    },
    identity::Keypair,
    mplex, noise, quic,
    relay::v2::client::transport::ClientTransport,
    swarm::derive_prelude::EitherOutput,
    tcp, yamux, PeerId, Transport,
};

use crate::config::NetworkConfig;

/// Creates a new [`UrsaTransport`].
///
/// Defaults to QUIC transport over TCP.
/// If QUIC fails to establish a connection, we fail over to TCP.
pub(crate) fn build_transport(
    keypair: &Keypair,
    // todo(botch): make some of the transport options configurable
    _config: &NetworkConfig,
    relay_transport: Option<ClientTransport>,
) -> Boxed<(PeerId, StreamMuxerBox)> {
    let id_keys = keypair;

    let tcp = {
        let tcp_config = tcp::Config::default().port_reuse(true);
        let tcp_transport = tcp::tokio::Transport::new(tcp_config.clone());

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

        if let Some(relay) = relay_transport {
            tcp_transport
                .or_transport(relay)
                .upgrade(upgrade::Version::V1)
                .authenticate(noise)
                .multiplex(mplex)
                .boxed()
        } else {
            tcp_transport
                .upgrade(upgrade::Version::V1)
                .authenticate(noise)
                .multiplex(mplex)
                .boxed()
        }
    };

    let quic = {
        let quic_config = quic::Config::new(keypair);
        quic::tokio::Transport::new(quic_config)
    };

    OrTransport::new(quic, tcp)
        .map(|either_output, _| match either_output {
            EitherOutput::First((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
            EitherOutput::Second((peer_id, muxer)) => (peer_id, StreamMuxerBox::new(muxer)),
        })
        .boxed()
}
