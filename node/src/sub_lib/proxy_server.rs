// Copyright (c) 2017-2019, Substratum LLC (https://substratum.net) and/or its affiliates. All rights reserved.
use crate::sub_lib::cryptde::PublicKey;
use crate::sub_lib::data_version::DataVersion;
use crate::sub_lib::dispatcher::InboundClientData;
use crate::sub_lib::dispatcher::StreamShutdownMsg;
use crate::sub_lib::hopper::{ExpiredCoresPackage, MessageType};
use crate::sub_lib::neighborhood::{ExpectedService, RouteQueryResponse};
use crate::sub_lib::peer_actors::BindMessage;
use crate::sub_lib::proxy_client::{ClientResponsePayload_0v1, DnsResolveFailure_0v1};
use crate::sub_lib::sequence_buffer::SequencedPacket;
use crate::sub_lib::set_consuming_wallet_message::SetConsumingWalletMessage;
use crate::sub_lib::stream_key::StreamKey;
use crate::sub_lib::versioned_data::VersionedData;
use actix::Message;
use actix::Recipient;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub const DEFAULT_MINIMUM_HOP_COUNT: usize = 3;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum ProxyProtocol {
    HTTP,
    TLS,
}

// TODO: Based on the way it's used, this struct should comprise two elements: one, a nested
// struct that contains all the small, quickly-cloned things, and the other the big,
// expensively-cloned SequencedPacket.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[allow(non_camel_case_types)]
pub struct ClientRequestPayload_0v1 {
    pub stream_key: StreamKey,
    pub sequenced_packet: SequencedPacket,
    pub target_hostname: Option<String>,
    pub target_port: u16,
    pub protocol: ProxyProtocol,
    pub originator_public_key: PublicKey,
}

impl Into<MessageType> for ClientRequestPayload_0v1 {
    fn into(self) -> MessageType {
        MessageType::ClientRequest(VersionedData::new(
            &crate::sub_lib::migrations::client_request_payload::MIGRATIONS,
            &self,
        ))
    }
}

impl ClientRequestPayload_0v1 {
    pub fn version() -> DataVersion {
        DataVersion::new(0, 0).expect("Internal Error")
    }
}

#[derive(Message, Debug, PartialEq, Eq)]
pub struct AddReturnRouteMessage {
    pub return_route_id: u32,
    pub expected_services: Vec<ExpectedService>,
    pub protocol: ProxyProtocol,
    pub server_name: Option<String>,
}

#[derive(Message, Debug, PartialEq)]
pub struct AddRouteMessage {
    pub stream_key: StreamKey,
    pub route: RouteQueryResponse,
}

#[derive(Clone)]
pub struct ProxyServerSubs {
    // ProxyServer will handle these messages:
    pub bind: Recipient<BindMessage>,
    pub from_dispatcher: Recipient<InboundClientData>,
    pub from_hopper: Recipient<ExpiredCoresPackage<ClientResponsePayload_0v1>>,
    pub dns_failure_from_hopper: Recipient<ExpiredCoresPackage<DnsResolveFailure_0v1>>,
    pub add_return_route: Recipient<AddReturnRouteMessage>,
    pub add_route: Recipient<AddRouteMessage>,
    pub stream_shutdown_sub: Recipient<StreamShutdownMsg>,
    pub set_consuming_wallet_sub: Recipient<SetConsumingWalletMessage>,
}

impl Debug for ProxyServerSubs {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "ProxyServerSubs")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sub_lib::proxy_server::ProxyServerSubs;
    use crate::test_utils::recorder::Recorder;
    use actix::Actor;

    #[test]
    fn proxy_server_subs_debug() {
        let recorder = Recorder::new().start();

        let subject = ProxyServerSubs {
            bind: recipient!(recorder, BindMessage),
            from_dispatcher: recipient!(recorder, InboundClientData),
            from_hopper: recipient!(recorder, ExpiredCoresPackage<ClientResponsePayload_0v1>),
            dns_failure_from_hopper: recipient!(
                recorder,
                ExpiredCoresPackage<DnsResolveFailure_0v1>
            ),
            add_return_route: recipient!(recorder, AddReturnRouteMessage),
            add_route: recipient!(recorder, AddRouteMessage),
            stream_shutdown_sub: recipient!(recorder, StreamShutdownMsg),
            set_consuming_wallet_sub: recipient!(recorder, SetConsumingWalletMessage),
        };

        assert_eq!(format!("{:?}", subject), "ProxyServerSubs");
    }
}
