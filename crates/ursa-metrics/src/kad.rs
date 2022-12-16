use crate::Recorder;
use libp2p_kad::{GetProvidersOk, GetRecordOk, KademliaEvent, QueryResult};
use metrics::Label;
use metrics::{histogram, increment_counter};

impl Recorder for KademliaEvent {
    fn record(&self) {
        match self {
            KademliaEvent::OutboundQueryProgressed { result, stats, .. } => {
                histogram!("kad_query_result_num_requests", stats.num_requests() as f64);
                histogram!("kad_query_result_num_success", stats.num_successes() as f64);
                histogram!("kad_query_result_num_failure", stats.num_failures() as f64);
                if let Some(duration) = stats.duration() {
                    histogram!("kad_query_result_duration", duration.as_secs_f64());
                }

                match result {
                    QueryResult::GetRecord(result) => match result {
                        Ok(v) => {
                            match v {
                                GetRecordOk::FoundRecord(_) => {
                                    increment_counter!("kad_query_result_get_record_ok")
                                }
                                GetRecordOk::FinishedWithNoAdditionalRecord { .. } => {}
                            }
                        }
                        Err(_) => increment_counter!("kad_query_result_get_record_err"),
                    },
                    QueryResult::GetClosestPeers(result) => match result {
                        Ok(v) => histogram!(
                            "kad_query_result_get_closest_peers_ok",
                            v.peers.len() as f64
                        ),
                        Err(_) => increment_counter!("kad_query_result_get_closest_peers_err"),
                    },
                    QueryResult::GetProviders(result) => match result {
                        Ok(v) => match v {
                            GetProvidersOk::FoundProviders { providers, .. } => {
                                histogram!(
                                    "kad_query_result_get_providers_ok",
                                    providers.len() as f64
                                )
                            }
                            GetProvidersOk::FinishedWithNoAdditionalRecord { .. } => {}
                        },
                        Err(_) => increment_counter!("kad_query_result_get_providers_err"),
                    },
                    QueryResult::Bootstrap(_)
                    | QueryResult::StartProviding(_)
                    | QueryResult::RepublishProvider(_)
                    | QueryResult::PutRecord(_)
                    | QueryResult::RepublishRecord(_) => {
                        // libp2p_metrics doesn't track these by default
                    }
                }
            }
            KademliaEvent::RoutingUpdated {
                is_new_peer,
                old_peer,
                bucket_range: (low, _high),
                ..
            } => {
                let bucket = low.ilog2().unwrap_or_default();
                let bucket_label = Label::new("bucket", bucket.to_string());

                if old_peer.is_some() {
                    increment_counter!(
                        "kad_routing_updated",
                        vec![RoutingAction::Evicted.into(), bucket_label.clone(),]
                    );
                }

                if *is_new_peer {
                    increment_counter!(
                        "kad_routing_updated",
                        vec![RoutingAction::Added.into(), bucket_label,]
                    );
                } else {
                    increment_counter!(
                        "kad_routing_updated",
                        vec![RoutingAction::Updated.into(), bucket_label,]
                    );
                }
            }
            KademliaEvent::InboundRequest { request } => {
                increment_counter!(
                    "kad_inbound_request",
                    vec![Label::new(
                        "request",
                        match request {
                            libp2p_kad::InboundRequest::FindNode { .. } => "find_node",
                            libp2p_kad::InboundRequest::GetProvider { .. } => "get_providers",
                            libp2p_kad::InboundRequest::AddProvider { .. } => "add_provider",
                            libp2p_kad::InboundRequest::GetRecord { .. } => "get_record",
                            libp2p_kad::InboundRequest::PutRecord { .. } => "put_record",
                        }
                    ),]
                );
            }
            KademliaEvent::UnroutablePeer { .. }
            | KademliaEvent::RoutablePeer { .. }
            | KademliaEvent::PendingRoutablePeer { .. } => {
                // libp2p_metrics doesn't track these by default
            }
        }
    }
}

enum RoutingAction {
    Added,
    Updated,
    Evicted,
}

impl From<RoutingAction> for Label {
    fn from(action: RoutingAction) -> Self {
        match action {
            RoutingAction::Added => Label::new("action", "added"),
            RoutingAction::Updated => Label::new("action", "updated"),
            RoutingAction::Evicted => Label::new("action", "evicted"),
        }
    }
}
