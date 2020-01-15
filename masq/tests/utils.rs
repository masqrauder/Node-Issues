// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use masq_lib::ui_gateway::{NodeFromUiMessage, NodeToUiMessage};
use masq_lib::ui_traffic_converter::UiTrafficConverter;
use websocket::sync::Server;
use std::net::{SocketAddr, TcpListener};
use masq_lib::utils::localhost;
use std::sync::{Arc, Mutex};

pub struct MockWebSocketsServer {
    port: u16,
    responses: Vec<String>,
}

pub struct StopHandle {
    server: Arc<Mutex<WsServer<NoTlsAcceptor, TcpListener>>>,
    mock: MockWebSocketsServer
}

impl MockWebSocketsServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            responses: vec![],
        }
    }

    pub fn queue_response (mut self, message: NodeToUiMessage) -> Self {
        self.responses.push (UiTrafficConverter::new_marshal_to_ui(message));
        self
    }

    pub fn queue_string (mut self, string: &str) -> Self {
        self.responses.push (string.to_string());
        self
    }

    pub fn start (mut self) -> StopHandle {
        let handle = StopHandle {
            server: Arc::new(Mutex::new(Server::bind(SocketAddr::new(localhost(), self.port)).unwrap())),
            mock: self
        };
        unimplemented!()
    }
}

impl StopHandle {
    pub fn stop (self) -> Vec<Result<NodeFromUiMessage, String>> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use masq_lib::ui_gateway::MessageTarget::ClientId;
    use masq_lib::messages::{UiSetup, UiSetupValue};
    use masq_lib::ui_connection::UiConnection;
    use masq_lib::utils::find_free_port;
    use masq_lib::messages::{FromMessageBody, ToMessageBody};

    #[test]
    fn two_in_two_out () {
        let port = find_free_port();
        let first_expected_response = NodeToUiMessage {
            target: ClientId(0),
            body: UiSetup {
                values: vec![
                    UiSetupValue{ name: "direction".to_string(), value: "to UI".to_string() }
                ]
            }.tmb(1234)
        };
        let stop_handle = MockWebSocketsServer::new(port)
            .queue_response (first_expected_response.clone())
            .queue_string ("}: Bad response :{")
            .start();
        let mut connection = UiConnection::new(port, "MASQNode-UIv2");
        let first_request_payload = UiSetup {
            values: vec![
                UiSetupValue{ name: "direction".to_string(), value: "from UI".to_string() }
            ]
        };

        let first_actual_response: UiSetup = connection.transact_with_context_id(first_request_payload.clone(), 1234).unwrap();
        connection.send_string("}: Bad request :{".to_string());
        let second_actual_response: Result<UiSetup, (u64, String)> = connection.receive();

        let requests = stop_handle.stop();
        assert_eq! (requests[0], Ok(NodeFromUiMessage {
            client_id: 0,
            body: first_request_payload.tmb(1234),
        }));
        assert_eq! ((first_actual_response, 1234), UiSetup::fmb(first_expected_response.body).unwrap());
        assert_eq! (requests[1], Err("}: Bad request :{".to_string()));
        assert_eq! (second_actual_response, Err((1, "booga".to_string())));
    }
}