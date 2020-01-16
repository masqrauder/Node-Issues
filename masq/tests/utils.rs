// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use masq_lib::ui_gateway::{NodeFromUiMessage, NodeToUiMessage};
use masq_lib::ui_traffic_converter::UiTrafficConverter;
use websocket::sync::Server;
use std::net::{SocketAddr, TcpListener};
use masq_lib::utils::localhost;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::thread;
use websocket::OwnedMessage;

pub struct MockWebSocketsServer {
    port: u16,
    responses: Vec<String>,
}

pub struct StopHandle {
    server_arc: Arc<Mutex<WsServer<NoTlsAcceptor, TcpListener>>>,
    requests_arc: Arc<Mutex<Vec<Result<NodeFromUiMessage, String>>>>,
    mock: MockWebSocketsServer,
    join_handle: JoinHandle<()>,
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
        let server_arc = Arc::new(Mutex::new(Server::bind(SocketAddr::new(localhost(), self.port)).unwrap()));
        let inner_server_arc = server_arc.clone();
        let requests_arc = Arc::new(Mutex::new(vec![]));
        let inner_requests_arc = requests_arc.clone();
        let join_handle = thread::spawn (move || {
            let mut server = inner_server_arc.lock().unwrap();
            let mut requests = inner_requests_arc.lock().unwrap();
            let upgrade = server.accept().unwrap();
            let mut client = upgrade.accept().unwrap();
            loop {
                match client.recv_message().unwrap() {
                    OwnedMessage::Text(json) => {
                        requests.push (match UiTrafficConverter::new_unmarshal_from_ui(json, 0) {
                            Ok(msg) => Ok(msg),
                            Err(e) => Err(json),
                        })
                    },
                    x => requests.push (Err(format!("{:?}", x))),
                }
            }
        });
        StopHandle {
            server_arc,
            requests_arc,
            mock: self,
            join_handle,
        }
    }
}

impl StopHandle {
    pub fn stop (self) -> Vec<Result<NodeFromUiMessage, String>> {
        // send stop signal here somehow
        let _ = self.join_handle.join();
        self.requests_arc.lock().unwrap().clone()
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