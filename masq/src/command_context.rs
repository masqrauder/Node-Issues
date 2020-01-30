// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use masq_lib::command::StdStreams;
use std::io::{Write, Stderr, Stdout, Stdin, Read};
use masq_lib::ui_gateway::{NodeFromUiMessage, NodeToUiMessage};
use masq_lib::ui_traffic_converter::UnmarshalError;
use masq_lib::messages::{ToMessageBody, FromMessageBody};
use crate::websockets_client::{NodeConversation, NodeConnection};
use std::io;

pub trait CommandContext {
    fn send (&mut self, message: NodeFromUiMessage) -> Result<(), String>;
    fn transact (&mut self, message: NodeFromUiMessage) -> Result<NodeToUiMessage, String>;
    fn stdin(&mut self) -> &mut Box<dyn Read>;
    fn stdout(&mut self) -> &mut Box<dyn Write>;
    fn stderr(&mut self) -> &mut Box<dyn Write>;
}

pub struct CommandContextReal {
    connection: NodeConnection,
    pub stdin: Box<dyn Read>,
    pub stdout: Box<dyn Write>,
    pub stderr: Box<dyn Write>,
}

impl CommandContext for CommandContextReal {

    fn send (&mut self, message: NodeFromUiMessage) -> Result<(), String> {
        let mut conversation = self.connection.start_conversation();
        conversation.send(message)
    }

    fn transact (&mut self, message: NodeFromUiMessage) -> Result<NodeToUiMessage, String> {
        let mut conversation = self.connection.start_conversation();
        match conversation.transact (message) {
            Err (e) => Err (e),
            Ok(ntum) => match ntum.body.payload {
                Err((code, msg)) => Err (format!("Daemon or Node reports error {:X}: {}", code, msg)),
                Ok (_) => Ok (ntum)
            }
        }
    }

    fn stdin(&mut self) -> &mut Box<dyn Read> {
        &mut self.stdin
    }

    fn stdout(&mut self) -> &mut Box<dyn Write> {
        &mut self.stdout
    }

    fn stderr(&mut self) -> &mut Box<dyn Write> {
        &mut self.stderr
    }
}

impl CommandContextReal {
    pub fn new (port: u16) -> Self {
        Self {
            connection: NodeConnection::new (port).expect ("Couldn't connect to Daemon or Node"),
            stdin: Box::new (io::stdin()),
            stdout: Box::new (io::stdout()),
            stderr: Box::new (io::stderr()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use masq_lib::utils::find_free_port;
    use masq_lib::test_utils::fake_stream_holder::{FakeStreamHolder, ByteArrayReader};
    use crate::test_utils::mock_websockets_server::MockWebSocketsServer;
    use masq_lib::messages::{UiSetup, UiSetupValue, UiShutdownOrder};
    use masq_lib::ui_gateway::MessageTarget::ClientId;
    use masq_lib::messages::ToMessageBody;
    use masq_lib::ui_gateway::MessageBody;
    use masq_lib::ui_gateway::MessagePath::TwoWay;
    use crate::websockets_client::nfum;

    #[test]
    fn works_when_everythings_fine() {
        let port = find_free_port();
        let mut holder = FakeStreamHolder::new();
        holder.stdin = ByteArrayReader::new (b"This is stdin.");
        let server = MockWebSocketsServer::new(port)
            .queue_response (NodeToUiMessage {
                target: ClientId(0),
                body: UiSetup {
                    values: vec![
                        UiSetupValue {
                            name: "Okay,".to_string(),
                            value: "I did.".to_string(),
                        }
                    ]
                }.tmb(1234)
            });
        let stop_handle = server.start();
        let mut subject = CommandContextReal::new (port);
        subject.stdin = Box::new (holder.stdin);
        subject.stdout = Box::new (holder.stdout);
        subject.stderr = Box::new (holder.stderr);

        subject.send (nfum(UiShutdownOrder {})).unwrap();
        let response = subject.transact (nfum(UiSetup {
                values: vec![
                    UiSetupValue {
                        name: "Say something".to_string(),
                        value: "to me.".to_string(),
                    }
                ]
            })).unwrap();
        let mut input = String::new();
        subject.stdin().read_to_string(&mut input).unwrap();
        write!(subject.stdout(), "This is stdout.").unwrap();
        write!(subject.stderr(), "This is stderr.").unwrap();

        assert_eq! (UiSetup::fmb(response.body).unwrap().0, UiSetup {
            values: vec![
                UiSetupValue {
                    name: "Okay,".to_string(),
                    value: "I did.".to_string(),
                }
            ]
        });
        assert_eq! (input, "This is stdin.".to_string());
        assert_eq! (holder.stdout.get_string(), "This is stdout.".to_string());
        assert_eq! (holder.stderr.get_string(), "This is stderr.".to_string());
        stop_handle.stop();
    }

    #[test]
    fn works_when_server_sends_payload_error() {
        let port = find_free_port();
        let server = MockWebSocketsServer::new(port)
            .queue_response (NodeToUiMessage {
                target: ClientId(0),
                body: MessageBody {
                    opcode: "setup".to_string(),
                    path: TwoWay(1234),
                    payload: Err((101, "booga".to_string()))
                }
            });
        let stop_handle = server.start();
        let mut holder = FakeStreamHolder::new();
        let mut streams = holder.streams();
        let mut subject = CommandContextReal::new (port);

        let response = subject.transact (nfum(UiSetup {
            values: vec![]
        }));

        assert_eq! (response, Err(format!("Daemon or Node reports error 65: booga")));
        stop_handle.stop();
    }

    #[test]
    fn works_when_server_sends_connection_error() {
        let port = find_free_port();
        let server = MockWebSocketsServer::new(port)
            .queue_string ("disconnect");
        let stop_handle = server.start();
        let mut holder = FakeStreamHolder::new();
        let mut subject = CommandContextReal::new (port);

        let response = subject.transact (nfum(UiSetup {
            values: vec![]
        }));

        assert_eq! (response, Err(format!("NoDataAvailable")));
    }
}
