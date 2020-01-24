// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use masq_lib::command::StdStreams;
use std::io::{Write, Read};
use masq_lib::ui_gateway::{NodeFromUiMessage, NodeToUiMessage};
use masq_lib::ui_traffic_converter::UnmarshalError;
use masq_lib::messages::{ToMessageBody, FromMessageBody};

pub struct CommandContext<'a> {
    streams: &'a StdStreams<'a>,
}

impl<'a> CommandContext<'a> {
    pub fn new (port: u16, streams: &'a mut StdStreams<'a>) -> Self {
        Self {
            streams
        }
    }

    pub fn transact<T: ToMessageBody, F: FromMessageBody> (&mut self, message: T) -> Result<Option<F>, UnmarshalError> {
        unimplemented!()
    }

    pub fn stdin(&mut self) -> &mut (dyn Read) {
        unimplemented!()
    }

    pub fn stdout(&mut self) -> &mut (dyn Write) {
        unimplemented!()
    }

    pub fn stderr(&mut self) -> &mut (dyn Write) {
        unimplemented!()
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

    #[test]
    fn can_be_constructed_when_everythings_fine() {
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

        let first_response = subject.transact (UiShutdownOrder {});
        let second_response = subject.transact (UiSetup {
                values: vec![
                    UiSetupValue {
                        name: "Say something".to_string(),
                        value: "to me.".to_string(),
                    }
                ]
            });
        let mut input = String::new();
        subject.stdin().read_to_string(&mut input).unwrap();
        let _ = write!(subject.stdout(), "This is stdout.");
        let _ = write!(subject.stderr(), "This is stderr.");

        assert_eq! (first_response, Ok(None));
        assert_eq! (second_response, Ok(Some (NodeToUiMessage {
            target: ClientId(0),
            body: UiSetup {
                values: vec![
                    UiSetupValue {
                        name: "Okay,".to_string(),
                        value: "I did.".to_string(),
                    }
                ]
            }.tmb(1234)
        })));
        assert_eq! (input, "This is stdin.".to_string());
        assert_eq! (holder.stdout.get_string(), "This is stdout.".to_string());
        assert_eq! (holder.stderr.get_string(), "This is stderr.".to_string());
    }
}