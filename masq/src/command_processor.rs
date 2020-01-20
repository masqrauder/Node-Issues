// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use std::fmt::Debug;
use masq_lib::ui_traffic_converter::UnmarshalError;
use masq_lib::ui_gateway::{NodeFromUiMessage, NodeToUiMessage};
use std::io::{Read, Write};
use masq_lib::command::StdStreams;
use crate::commands::{Command, CommandError};

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
        panic!("masq was not properly initialized")
    }

    fn shutdown(&mut self) {
        panic!("masq was not properly initialized")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_factory::SetupCommand;
    use crate::commands::SetupCommand;

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
