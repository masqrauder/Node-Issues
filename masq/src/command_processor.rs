// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use std::fmt::Debug;
use masq_lib::ui_traffic_converter::UnmarshalError;
use masq_lib::ui_gateway::{NodeFromUiMessage, NodeToUiMessage};
use std::io::{Read, Write};
use masq_lib::command::StdStreams;

#[derive (Debug, PartialEq)]
pub enum CommandError {
    Transaction(UnmarshalError),
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

pub trait Command: Debug {
    fn execute<'a>(&self, context: &mut Box<dyn CommandContext<'a> + 'a>) -> Result<(), CommandError>;
}

pub trait CommandProcessorFactory {
    fn make(&self, streams: &mut StdStreams<'_>, args: &[String]) -> Box<dyn CommandProcessor>;
}

pub struct CommandProcessorFactoryReal {}

impl CommandProcessorFactory for CommandProcessorFactoryReal {
    fn make(&self, streams: &mut StdStreams<'_>, args: &[String]) -> Box<dyn CommandProcessor> {
        unimplemented!()
    }
}

pub trait CommandProcessor {
    fn process (&mut self, command: Box<dyn Command>) -> Result<(), CommandError>;
    fn shutdown (&mut self);
}

pub struct CommandProcessorReal {

}

impl CommandProcessor for CommandProcessorReal {
    fn process(&mut self, command: Box<dyn Command>) -> Result<(), CommandError> {
        unimplemented!()
    }

    fn shutdown(&mut self) {
        unimplemented!()
    }
}

impl CommandProcessorReal {
    pub fn new(streams: &mut StdStreams<'_>, args: &Vec<String>) -> Self {
        unimplemented!()
    }
}

pub struct CommandProcessorNull {}

impl CommandProcessor for CommandProcessorNull {
    fn process(&mut self, command: Box<dyn Command>) -> Result<(), CommandError> {
        unimplemented!()
    }

    fn shutdown(&mut self) {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_factory::SetupCommand;

    #[test]
    #[should_panic(expected = "masq was not properly initialized")]
    fn null_command_processor_process_panics_properly() {
        let mut subject = CommandProcessorNull{};

        subject.process (Box::new (SetupCommand{values: vec![]})).unwrap();
    }

    #[test]
    #[should_panic(expected = "masq was not properly initialized")]
    fn null_command_processor_shutdown_panics_properly() {
        let mut subject = CommandProcessorNull{};

        subject.shutdown ();
    }
}
