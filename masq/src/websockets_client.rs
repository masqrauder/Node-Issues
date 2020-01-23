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

pub struct NodeConnectionInner {
    next_context_id: u64,
    client: Client<TcpStream>,
}

pub struct NodeConnection {
    inner_arc: Arc<Mutex<NodeConnectionInner>>,
}

impl Drop for NodeConnection {
    fn drop(&mut self) {
        eprintln! ("Complete me!");
    }
}

impl NodeConnection {
    pub fn new(port: u16) -> Result<NodeConnection, String> {
        let mut builder = ClientBuilder::new(format!("ws://{}:{}", localhost(), port).as_str()).expect ("Bad URL");
        let client = match builder.add_protocol(NODE_UI_PROTOCOL).connect_insecure() {
            Err(e) => return Err(format!("No Node or Daemon is listening on port {}: {:?}", port, e)),
            Ok(c) => c,
        };
        let inner_arc = Arc::new(Mutex::new(NodeConnectionInner {
            client,
            next_context_id: 0,
        }));
        Ok(NodeConnection { inner_arc })
    }

    pub fn start_conversation(&self) -> NodeConversation {
        unimplemented!()
    }
}

pub struct NodeConversation {
    context_id: u64,
    connection: Arc<Mutex<NodeConnectionInner>>
}

impl Drop for NodeConversation {
    fn drop(&mut self) {
        unimplemented!()
    }
}

impl NodeConversation {
    pub fn send<T: ToMessageBody>(&mut self, payload: T) -> Result<(), String> {
        let outgoing_msg = NodeFromUiMessage {
            client_id: 0, // irrelevant: will be replaced on the other end
            body: payload.tmb(self.context_id),
        };
        let outgoing_msg_json = UiTrafficConverter::new_marshal_from_ui(outgoing_msg);
        self.send_string(outgoing_msg_json)
    }

    pub fn establish_receiver<F>(&mut self, context_id: u64, receiver: F) -> Result<(), String> where F: Fn() -> NodeToUiMessage {
        unimplemented!();
    }

    pub fn transact<S: ToMessageBody, R: FromMessageBody>(
        &mut self,
        payload: S,
    ) -> Result<Result<R, (u64, String)>, String> {
        if !payload.is_two_way() {
            return Err(format!("'{}' message is one-way only; can't transact() with it", payload.opcode()))
        }
        self.send(payload);
        self.receive::<R>()
    }

    fn send_string(&mut self, string: String) -> Result<(), String> {
//        let mut client = &self.connection.lock().expect ("Connection poisoned").client;
//        if let Err(e) = client.send_message(&OwnedMessage::Text(string)) {
//            Err(format!("{:?}", e))
//        }
//        else {
//            Ok (())
//        }
        unimplemented!()
    }

    fn receive<T: FromMessageBody>(&mut self) -> Result<Result<T, (u64, String)>, String> {
//        let mut client = &self.connection.lock().expect ("Connection poisoned").client;
//        let incoming_msg = client.recv_message();
//        let incoming_msg_json = match incoming_msg {
//            Ok(OwnedMessage::Text(json)) => json,
//            Ok(x) => return Err(format!("Expected text; received {:?}", x)),
//            Err (e) => return Err(format!("{:?}", e)),
//        };
//        let incoming_msg = match UiTrafficConverter::new_unmarshal_to_ui(&incoming_msg_json, ClientId(0)) {
//            Ok(m) => m,
//            Err(e) => return Err(format! ("Deserialization problem: {:?}", e)),
//        };
//        let opcode = incoming_msg.body.opcode.clone();
//        let result: Result<(T, u64), UiMessageError> = T::fmb(incoming_msg.body);
//        match result {
//            Ok((payload, _)) => Ok(Ok(payload)),
//            Err(UiMessageError::PayloadError(code, message)) => Ok(Err((code, message))),
//            Err(e) => return Err(format!("Deserialization problem for {}: {:?}", opcode, e)),
//        }
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use masq_lib::utils::find_free_port;
    use crate::test_utils::mock_websockets_server::MockWebSocketsServer;

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
}
