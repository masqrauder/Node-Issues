// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use std::sync::{Arc, Mutex};

pub struct MockWebSocketsServer {

}

pub struct StopHandle {

}

impl MockWebSocketsServer {
    pub fn new(port: u16) -> Self {
        Self {

        }
    }

    pub fn queue_response (mut self, message: Box<dyn NodeToUiMessage>) -> Self {
        unimplemented!()
    }

    pub fn start (&self) -> StopHandle {
        unimplemented!()
    }
}

impl StopHandle {
    pub fn stop (self) -> Vec<Box<NodeFromUiMessage>> {
        unimplemented!()
    }
}
