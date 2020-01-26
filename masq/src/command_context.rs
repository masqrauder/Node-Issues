// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use masq_lib::command::StdStreams;
use std::io::{Write};
use masq_lib::ui_gateway::{NodeFromUiMessage, NodeToUiMessage};
use masq_lib::ui_traffic_converter::UnmarshalError;
use masq_lib::messages::{ToMessageBody, FromMessageBody};
use crate::websockets_client::{NodeConversation, NodeConnection};

pub struct CommandContext<'a> {
    connection: NodeConnection,
    streams: &'a mut StdStreams<'a>,
}

impl<'a> CommandContext<'a> {
    pub fn new (port: u16, streams: &'a mut StdStreams<'a>) -> Self {
        Self {
            connection: NodeConnection::new (port).expect ("Couldn't connect to Daemon or Node"),
            streams
        }
    }

    pub fn send<T: ToMessageBody> (&mut self, message: T) -> Result<(), String> {
        let mut conversation = self.connection.start_conversation();
        conversation.send(message)
    }

    pub fn transact<T: ToMessageBody, F: FromMessageBody> (&mut self, message: T) -> Result<F, String> {
        let mut conversation = self.connection.start_conversation();
        match conversation.transact (message) {
            Ok(Ok(response)) => Ok(response),
            Ok(Err((code, message))) => Err(format!("Daemon or Node reports error {:X}: {}", code, message)),
            Err(msg) => Err(msg),
        }
    }

    pub fn read_stdin(&mut self) -> String {
        let mut input = String::new();
        self.streams.stdin.read_to_string(&mut input).expect ("Couldn't read standard input");
        input
    }

    pub fn write_stdout(&mut self, string: &str) {
        write!(self.streams.stdout, "{}", string).expect ("write! to stdout failed");
    }

    pub fn write_stderr(&mut self, string: &str) {
        write!(self.streams.stderr, "{}", string).expect ("write! to stderr failed");
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
        let mut std_streams = holder.streams();
        let mut subject = CommandContext::new (port, &mut std_streams);

        subject.send (UiShutdownOrder {}).unwrap();
        let response: Result<UiSetup, String> = subject.transact (UiSetup {
                values: vec![
                    UiSetupValue {
                        name: "Say something".to_string(),
                        value: "to me.".to_string(),
                    }
                ]
            });
        let input = subject.read_stdin();
        subject.write_stdout("This is stdout.");
        subject.write_stderr("This is stderr.");

        assert_eq! (response, Ok(UiSetup {
            values: vec![
                UiSetupValue {
                    name: "Okay,".to_string(),
                    value: "I did.".to_string(),
                }
            ]
        }));
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
        let mut subject = CommandContext::new (port, &mut streams);

        let response: Result<UiSetup, String> = subject.transact (UiSetup {
            values: vec![]
        });

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
        let mut streams = holder.streams();
        let mut subject = CommandContext::new (port, &mut streams);

        let response: Result<UiSetup, String> = subject.transact (UiSetup {
            values: vec![]
        });

        assert_eq! (response, Err(format!("NoDataAvailable")));
    }
}
