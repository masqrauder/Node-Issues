// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use actix::Message;

pub const DEFAULT_UI_PORT: u16 = 5333;

#[derive(PartialEq, Clone, Debug)]
pub enum MessageTarget {
    ClientId(u64),
    AllClients,
}

#[derive(PartialEq, Clone, Debug)]
pub enum MessagePath {
    OneWay,
    TwoWay(u64), // context_id
}

#[derive(PartialEq, Clone, Debug)]
pub struct MessageBody {
    pub opcode: String,
    pub path: MessagePath,
    pub payload: Result<String, (u64, String)>, // <success payload as JSON, (error code, error message)>
}

#[derive(Message, PartialEq, Clone, Debug)]
pub struct NodeFromUiMessage {
    pub client_id: u64,
    pub body: MessageBody,
}

#[derive(Message, PartialEq, Clone, Debug)]
pub struct NodeToUiMessage {
    pub target: MessageTarget,
    pub body: MessageBody,
}
