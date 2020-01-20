// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use masq_lib::command::StdStreams;
use std::io::{Write, Read};
use masq_lib::ui_gateway::{NodeFromUiMessage, NodeToUiMessage};
use masq_lib::ui_traffic_converter::UnmarshalError;

pub trait CommandContextFactory {
    fn make (&self, port: u16, streams: &StdStreams<'_>);
}

pub struct CommandContextFactoryReal {}

impl CommandContextFactory for CommandContextFactoryReal {
    fn make(&self, port: u16, streams: &StdStreams<'_>) {
        unimplemented!()
    }
}

pub trait CommandContext<'a> {
    fn transact (&mut self, message: NodeFromUiMessage) -> Result<Option<NodeToUiMessage>, UnmarshalError>;
    fn stdin (&mut self) -> &mut (dyn Read);
    fn stdout (&mut self) -> &mut (dyn Write);
    fn stderr (&mut self) -> &mut (dyn Write);
}

pub struct CommandContextReal<'a> {
    streams: &'a StdStreams<'a>,
}

impl<'a> CommandContext<'a> for CommandContextReal<'_> {
    fn transact(&mut self, message: NodeFromUiMessage) -> Result<Option<NodeToUiMessage>, UnmarshalError> {
        unimplemented!()
    }

    fn stdin(&mut self) -> &mut (dyn Read) {
        unimplemented!()
    }

    fn stdout(&mut self) -> &mut (dyn Write) {
        unimplemented!()
    }

    fn stderr(&mut self) -> &mut (dyn Write) {
        unimplemented!()
    }
}

impl<'a> CommandContextReal<'a> {
    fn new (port: u16, streams: &'a mut StdStreams<'a>) -> Self {
        Self {
            streams
        }
    }
}
