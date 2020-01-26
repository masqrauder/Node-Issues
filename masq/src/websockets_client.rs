// Copyright (c) 2019-2020, MASQ (https://masq.ai). All rights reserved.

use std::net::TcpStream;
use websocket::sync::Client;
use websocket::{ClientBuilder, OwnedMessage};
use std::sync::{Mutex, Arc};
use masq_lib::utils::localhost;
use masq_lib::messages::{NODE_UI_PROTOCOL, ToMessageBody, FromMessageBody, UiMessageError};
use masq_lib::ui_gateway::{NodeFromUiMessage, NodeToUiMessage};
use masq_lib::ui_traffic_converter::UiTrafficConverter;
use masq_lib::ui_gateway::MessageTarget::ClientId;

pub const BROADCAST_CONTEXT_ID: u64 = 0;

pub struct NodeConnectionInner {
    next_context_id: u64,
    client: Client<TcpStream>,
}

pub struct NodeConnection {
    inner_arc: Arc<Mutex<NodeConnectionInner>>,
}

impl Drop for NodeConnection {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.inner_arc.lock() {
            let _ = guard.client.send_message(&OwnedMessage::Close(None));
        }
    }
}

impl NodeConnection {
    pub fn new(port: u16) -> Result<NodeConnection, String> {
        let builder = ClientBuilder::new(format!("ws://{}:{}", localhost(), port).as_str()).expect ("Bad URL");
        let client = match builder.add_protocol(NODE_UI_PROTOCOL).connect_insecure() {
            Err(e) => return Err(format!("No Node or Daemon is listening on port {}: {:?}", port, e)),
            Ok(c) => c,
        };
        let inner_arc = Arc::new(Mutex::new(NodeConnectionInner {
            client,
            next_context_id: BROADCAST_CONTEXT_ID + 1,
        }));
        Ok(NodeConnection { inner_arc })
    }

    pub fn start_conversation(&self) -> NodeConversation {
        let inner_arc = self.inner_arc.clone();
        let context_id = {
            let mut inner = inner_arc.lock().expect("NodeConnection is poisoned");
            let context_id = inner.next_context_id;
            inner.next_context_id += 1;
            context_id
        };
        let conversation = NodeConversation {
            context_id,
            inner_arc,
        };
        conversation
    }

    pub fn establish_broadcast_receiver<F>(&self, receiver: F) -> Result<(), String> where F: Fn() -> NodeToUiMessage {
        unimplemented!();
    }
}

pub struct NodeConversation {
    context_id: u64,
    inner_arc: Arc<Mutex<NodeConnectionInner>>
}

impl Drop for NodeConversation {
    fn drop(&mut self) {
        // TODO: When the client goes asynchronous, this will have to delete the conversation from the connection's map.
    }
}

impl NodeConversation {
    pub fn context_id(&self) -> u64 {
        self.context_id
    }

    pub fn send<T: ToMessageBody>(&mut self, payload: T) -> Result<(), String> {
        let outgoing_msg = NodeFromUiMessage {
            client_id: 0, // irrelevant: will be replaced on the other end
            body: payload.tmb(self.context_id),
        };
        let outgoing_msg_json = UiTrafficConverter::new_marshal_from_ui(outgoing_msg);
        self.send_string(outgoing_msg_json)
    }

    pub fn establish_receiver<F>(mut self, receiver: F) -> Result<(), String> where F: Fn() -> NodeToUiMessage {
        unimplemented!();
    }

    pub fn transact<S: ToMessageBody, R: FromMessageBody>(
        &mut self,
        payload: S,
    ) -> Result<Result<R, (u64, String)>, String> {
        if !payload.is_two_way() {
            return Err(format!("'{}' message is one-way only; can't transact() with it", payload.opcode()))
        }
        if let Err(e) = self.send(payload) {
            return Err(e) // Don't know how to drive this line
        }
        self.receive::<R>()
    }

    fn send_string(&mut self, string: String) -> Result<(), String> {
        let client = &mut self.inner_arc.lock().expect ("Connection poisoned").client;
        if let Err(e) = client.send_message(&OwnedMessage::Text(string)) {
            Err(format!("{:?}", e))
        }
        else {
            Ok (())
        }
    }

    fn receive<T: FromMessageBody>(&mut self) -> Result<Result<T, (u64, String)>, String> {
        let client = &mut self.inner_arc.lock().expect ("Connection poisoned").client;
        let incoming_msg = client.recv_message();
        let incoming_msg_json = match incoming_msg {
            Ok(OwnedMessage::Text(json)) => json,
            Ok(x) => return Err(format!("Expected text; received {:?}", x)),
            Err (e) => return Err(format!("{:?}", e)),
        };
        let incoming_msg = match UiTrafficConverter::new_unmarshal_to_ui(&incoming_msg_json, ClientId(0)) {
            Ok(m) => m,
            Err(e) => return Err(format! ("Deserialization problem: {:?}", e)),
        };
        let opcode = incoming_msg.body.opcode.clone();
        let result: Result<(T, u64), UiMessageError> = T::fmb(incoming_msg.body);
        match result {
            Ok((payload, _)) => Ok(Ok(payload)),
            Err(UiMessageError::PayloadError(code, message)) => Ok(Err((code, message))),
            Err(e) => return Err(format!("Deserialization problem for {}: {:?}", opcode, e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use masq_lib::utils::find_free_port;
    use crate::test_utils::mock_websockets_server::MockWebSocketsServer;
    use masq_lib::messages::{UiSetup, UiSetupValue, UiShutdownOrder};
    use std::time::Duration;
    use std::thread;
    use masq_lib::ui_gateway::{MessageBody, MessagePath};

    #[test]
    fn connection_works_when_no_server_exists() {
        let port = find_free_port();

        let err_msg = NodeConnection::new (port).err().unwrap();

        assert_eq! (err_msg.starts_with(&format!("No Node or Daemon is listening on port {}: ", port)), true, "{}", err_msg);
    }

    #[test]
    fn connection_works_when_protocol_doesnt_match() {
        let port = find_free_port();
        let mut server = MockWebSocketsServer::new(port);
        server.protocol = "Booga".to_string();
        server.start();

        let err_msg = NodeConnection::new (port).err().unwrap();

        assert_eq! (err_msg.starts_with(&format!("No Node or Daemon is listening on port {}: ", port)), true, "{}", err_msg);
    }

    #[test]
    fn dropping_connection_sends_a_close() {
        let port = find_free_port();
        let server = MockWebSocketsServer::new(port);
        let stop_handle = server.start();

        {
            let _ = NodeConnection::new(port).unwrap();
        }

        let results = stop_handle.stop();
        assert_eq! (results, vec![
            Err("Close(None)".to_string())
        ])
    }

    #[test]
    fn cant_transact_with_a_one_way_message() {
        let port = find_free_port();
        let server = MockWebSocketsServer::new(port);
        let stop_handle = server.start();
        let connection = NodeConnection::new(port).unwrap();
        let mut subject = connection.start_conversation();

        let result: Result<Result<UiShutdownOrder, (u64, String)>, String> = subject.transact (UiShutdownOrder {});

        assert_eq! (result, Err("'shutdownOrder' message is one-way only; can't transact() with it".to_string()));
        stop_handle.stop();
    }

    #[test]
    fn handles_connection_dropped_by_server_before_receive() {
        let port = find_free_port();
        let server = MockWebSocketsServer::new(port)
            .queue_string ("disconnect"); // magic value that causes disconnection
        let _ = server.start();
        let connection = NodeConnection::new(port).unwrap();
        let mut subject = connection.start_conversation();

        let result: Result<Result<UiSetup, (u64, String)>, String> = subject.transact (UiSetup {values: vec![]});

        assert_eq! (result, Err("NoDataAvailable".to_string()));
    }

    #[test]
    fn handles_connection_dropped_by_server_before_send() {
        let port = find_free_port();
        let server = MockWebSocketsServer::new(port)
            .queue_string ("disconnect"); // magic value that causes disconnection
        let stop_handle = server.start();
        let connection = NodeConnection::new(port).unwrap();
        let mut subject = connection.start_conversation();
        let _: Result<Result<UiSetup, (u64, String)>, String> = subject.transact (UiSetup {values: vec![]});
        let _ = subject.send (UiShutdownOrder {}); // dunno why this doesn't blow up

        let result = subject.send (UiShutdownOrder {}).err().unwrap();

        assert! (result.contains ("BrokenPipe"));
    }

    #[test]
    fn handles_being_sent_something_other_than_text() {
        let port = find_free_port();
        let server = MockWebSocketsServer::new(port)
            .queue_string ("disconnect"); // magic value that causes disconnection
        let stop_handle = server.start();
        let connection = NodeConnection::new(port).unwrap();
        let mut subject = connection.start_conversation();
        stop_handle.stop();

        let result: Result<Result<UiSetup, (u64, String)>, String> = subject.receive ();

        if let Err(err_msg) = result {
            assert_eq!(err_msg, "Expected text; received Close(None)".to_string());
        }
        else {
            assert!(false, "Expected Close(None); got {:?}", result);
        }
    }

    #[test]
    fn handles_being_sent_bad_syntax() {
        let port = find_free_port();
        let server = MockWebSocketsServer::new(port)
            .queue_string ("} -- bad syntax -- {");
        let stop_handle = server.start();
        let connection = NodeConnection::new(port).unwrap();
        let mut subject = connection.start_conversation();

        let result: Result<Result<UiSetup, (u64, String)>, String> = subject.transact (UiSetup {values: vec![]});

        assert_eq! (result, Err("Deserialization problem: Critical(JsonSyntaxError(\"Error(\\\"expected value\\\", line: 1, column: 1)\"))".to_string()));
    }

    #[test]
    fn handles_being_sent_a_payload_error() {
        let port = find_free_port();
        let server = MockWebSocketsServer::new(port)
            .queue_response (NodeToUiMessage {
                target: ClientId(0),
                body: MessageBody {
                    opcode: "setup".to_string(),
                    path: MessagePath::TwoWay(1),
                    payload: Err((101, "booga".to_string()))
                }
            });
        let stop_handle = server.start();
        let connection = NodeConnection::new(port).unwrap();
        let mut subject = connection.start_conversation();

        let result: Result<Result<UiSetup, (u64, String)>, String> = subject.transact (UiSetup {values: vec![]});

        assert_eq! (result, Ok(Err((101, "booga".to_string()))));
    }

    #[test]
    fn handles_being_sent_unrecognized_message() {
        let port = find_free_port();
        let server = MockWebSocketsServer::new(port)
            .queue_response (NodeToUiMessage {
                target: ClientId(0),
                body: MessageBody {
                    opcode: "booga".to_string(),
                    path: MessagePath::TwoWay(1),
                    payload: Err((101, "booga".to_string()))
                }
            });
        let stop_handle = server.start();
        let connection = NodeConnection::new(port).unwrap();
        let mut subject = connection.start_conversation();

        let result: Result<Result<UiSetup, (u64, String)>, String> = subject.transact (UiSetup {values: vec![]});

        assert_eq! (result, Err("Deserialization problem for booga: BadOpcode".to_string()));
    }

    #[test]
    fn single_cycle_conversation_works_as_expected() {
        let port = find_free_port();
        let server = MockWebSocketsServer::new(port)
            .queue_response (NodeToUiMessage {
                target: ClientId(0),
                body: UiSetup {
                    values: vec![
                        UiSetupValue::new ("type", "response")
                    ]
                }.tmb(1)
            });
        let stop_handle = server.start();
        let connection = NodeConnection::new(port).unwrap();
        let mut subject = connection.start_conversation();

        let response: UiSetup = subject.transact (UiSetup {
            values: vec![
                UiSetupValue::new ("type", "request")
            ]
        }).unwrap().unwrap();

        let requests = stop_handle.stop();
        assert_eq! (requests, vec![Ok(NodeFromUiMessage {
            client_id: 0,
            body: UiSetup {
                values: vec![UiSetupValue::new ("type", "request")]
            }.tmb(1)
        })]);
        assert_eq! (response, UiSetup {
            values: vec![UiSetupValue::new ("type", "response")]
        })
    }

    #[test]
    #[ignore] // Unignore this when it's time to go multithreaded
    fn overlapping_conversations_work_as_expected() {
        let port = find_free_port();
        let server = MockWebSocketsServer::new(port)
            .queue_response (NodeToUiMessage {
                target: ClientId(0),
                body: UiSetup {
                    values: vec![
                        UiSetupValue::new ("type", "conversation 2 response")
                    ]
                }.tmb(2)
            })
            .queue_response (NodeToUiMessage {
                target: ClientId(0),
                body: UiSetup {
                    values: vec![
                        UiSetupValue::new ("type", "conversation 1 response")
                    ]
                }.tmb(1)
            });
        let stop_handle = server.start();
        let connection = NodeConnection::new(port).unwrap();
        let mut subject1 = connection.start_conversation();
        let mut subject2 = connection.start_conversation();

        let response1: UiSetup = subject1.transact (UiSetup {
            values: vec![
                UiSetupValue::new ("type", "conversation 1 request")
            ]
        }).unwrap().unwrap();
        let response2: UiSetup = subject2.transact (UiSetup {
            values: vec![
                UiSetupValue::new ("type", "conversation 2 request")
            ]
        }).unwrap().unwrap();

        assert_eq! (subject1.context_id(), 1);
        assert_eq! (subject2.context_id(), 2);
        let requests = stop_handle.stop();
        assert_eq! (requests, vec![
            Ok(NodeFromUiMessage {
                client_id: 0,
                body: UiSetup {
                    values: vec![UiSetupValue::new ("type", "conversation 1 request")]
                }.tmb(1)
            }),
            Ok(NodeFromUiMessage {
                client_id: 0,
                body: UiSetup {
                    values: vec![UiSetupValue::new ("type", "conversation 2 request")]
                }.tmb(2)
            }),
        ]);
        assert_eq! (response1, UiSetup {
            values: vec![UiSetupValue::new ("type", "conversation 1 response")]
        });
        assert_eq! (response2, UiSetup {
            values: vec![UiSetupValue::new ("type", "conversation 2 response")]
        });
    }
}
