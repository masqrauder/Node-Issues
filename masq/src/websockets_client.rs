// Copyright (c) 2019-2020, MASQ (https://masq.ai). All rights reserved.

use std::net::TcpStream;
use websocket::sync::Client;
use websocket::{ClientBuilder, OwnedMessage};
use std::sync::{Mutex, Arc};
use masq_lib::utils::localhost;
use masq_lib::messages::{NODE_UI_PROTOCOL, ToMessageBody, FromMessageBody, UiMessageError};
use masq_lib::ui_gateway::{NodeFromUiMessage, NodeToUiMessage, MessageBody};
use masq_lib::ui_traffic_converter::UiTrafficConverter;
use masq_lib::ui_gateway::MessageTarget::ClientId;
use masq_lib::ui_gateway::MessagePath::{OneWay, TwoWay};

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

    pub fn send(&mut self, outgoing_msg: NodeFromUiMessage) -> Result<(), String> {
        let outgoing_msg_json = UiTrafficConverter::new_marshal_from_ui(outgoing_msg);
        self.send_string(outgoing_msg_json)
    }

    pub fn establish_receiver<F>(mut self, receiver: F) -> Result<(), String> where F: Fn() -> NodeToUiMessage {
        unimplemented!();
    }

    pub fn transact(
        &mut self,
        mut outgoing_msg: NodeFromUiMessage,
    ) -> Result<NodeToUiMessage, String> {
        if outgoing_msg.body.path == OneWay {
            return Err(format!("'{}' message is one-way only; can't transact() with it", outgoing_msg.body.opcode))
        }
        else {
            outgoing_msg.body.path = TwoWay(self.context_id());
        }
        if let Err(e) = self.send(outgoing_msg) {
            return Err(e) // Don't know how to drive this line
        }
        self.receive()
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

    fn receive(&mut self) -> Result<NodeToUiMessage, String> {
        let client = &mut self.inner_arc.lock().expect ("Connection poisoned").client;
        let incoming_msg = client.recv_message();
        let incoming_msg_json = match incoming_msg {
            Ok(OwnedMessage::Text(json)) => json,
            Ok(x) => return Err(format!("Expected text; received {:?}", x)),
            Err (e) => return Err(format!("{:?}", e)),
        };
        match UiTrafficConverter::new_unmarshal_to_ui(&incoming_msg_json, ClientId(0)) {
            Ok(m) => Ok(m),
            Err(e) => Err(format! ("Deserialization problem: {:?}", e)),
        }
    }
}

// Warning: this function does not properly set the context_id field.
pub fn nfum<T: ToMessageBody> (tmb: T) -> NodeFromUiMessage {
    NodeFromUiMessage {
        client_id: 0,
        body: tmb.tmb(0)
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
    use masq_lib::ui_gateway::MessagePath::TwoWay;

    pub fn nftm1<T: ToMessageBody> (tmb: T) -> NodeToUiMessage {
        assert_eq! (tmb.is_two_way(), false);
        NodeToUiMessage {
            target: ClientId(0),
            body: tmb.tmb(0)
        }
    }

    pub fn nftm2<T: ToMessageBody> (context_id: u64, tmb: T) -> NodeToUiMessage {
        assert_eq! (tmb.is_two_way(), true);
        NodeToUiMessage {
            target: ClientId(0),
            body: tmb.tmb(context_id)
        }
    }

    pub fn nftme1(opcode: &str, code: u64, msg: &str) -> NodeToUiMessage {
        NodeToUiMessage {
            target: ClientId(0),
            body: MessageBody {
                opcode: opcode.to_string(),
                path: OneWay,
                payload: Err ((code, msg.to_string()))
            }
        }
    }

    pub fn nftme2(opcode: &str, context_id: u64, code: u64, msg: &str) -> NodeToUiMessage {
        NodeToUiMessage {
            target: ClientId(0),
            body: MessageBody {
                opcode: opcode.to_string(),
                path: TwoWay(context_id),
                payload: Err ((code, msg.to_string()))
            }
        }
    }

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

        let result = subject.transact (nfum(UiShutdownOrder{}));

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

        let result = subject.transact (nfum(UiSetup {values: vec![]}));

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
        let _ = subject.transact (nfum(UiSetup {values: vec![]}));
        let _ = subject.send (nfum(UiShutdownOrder {})); // dunno why this doesn't blow up

        let result = subject.send (nfum(UiShutdownOrder {})).err().unwrap();

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

        let result = subject.receive ();

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

        let result = subject.transact (nfum(UiSetup {values: vec![]}));

        stop_handle.stop();
        assert_eq! (result, Err("Deserialization problem: Critical(JsonSyntaxError(\"Error(\\\"expected value\\\", line: 1, column: 1)\"))".to_string()));
    }

    #[test]
    fn handles_being_sent_a_payload_error() {
        let port = find_free_port();
        let server = MockWebSocketsServer::new(port)
            .queue_response (nftme2("setup", 1, 101, "booga"));
        let stop_handle = server.start();
        let connection = NodeConnection::new(port).unwrap();
        let mut subject = connection.start_conversation();

        let result = subject.transact (nfum(UiSetup {values: vec![]})).unwrap();

        stop_handle.stop();
        assert_eq! (result.body.payload, Err((101, "booga".to_string())));
    }

    #[test]
    fn single_cycle_conversation_works_as_expected() {
        let port = find_free_port();
        let server = MockWebSocketsServer::new(port)
            .queue_response (nftm2(1, UiSetup {
                values: vec![
                    UiSetupValue::new ("type", "response")
                ]
            }));
        let stop_handle = server.start();
        let connection = NodeConnection::new(port).unwrap();
        let mut subject = connection.start_conversation();

        let response_body = subject.transact (nfum(UiSetup {
            values: vec![
                UiSetupValue::new ("type", "request")
            ]
        })).unwrap().body;

        let response = UiSetup::fmb(response_body).unwrap();
        let requests = stop_handle.stop();
        assert_eq! (requests, vec![Ok(NodeFromUiMessage {
            client_id: 0,
            body: UiSetup {
                values: vec![UiSetupValue::new ("type", "request")]
            }.tmb(1)
        })]);
        assert_eq! (response, (UiSetup {
            values: vec![UiSetupValue::new ("type", "response")]
        }, 1));
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

        let response1_body = subject1.transact (nfum(UiSetup {
            values: vec![
                UiSetupValue::new ("type", "conversation 1 request")
            ]
        })).unwrap().body;
        let response2_body = subject2.transact (nfum(UiSetup {
            values: vec![
                UiSetupValue::new ("type", "conversation 2 request")
            ]
        })).unwrap().body;

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
        assert_eq! (response1_body.path, TwoWay(1));
        assert_eq! (UiSetup::fmb(response1_body).unwrap().0, UiSetup {
            values: vec![UiSetupValue::new ("type", "conversation 1 response")]
        });
        assert_eq! (response2_body.path, TwoWay(2));
        assert_eq! (UiSetup::fmb(response2_body).unwrap().0, UiSetup {
            values: vec![UiSetupValue::new ("type", "conversation 2 response")]
        });
    }
}
