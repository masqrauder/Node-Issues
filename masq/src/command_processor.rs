// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use std::fmt::Debug;
use masq_lib::ui_traffic_converter::UnmarshalError;
use masq_lib::ui_gateway::{NodeFromUiMessage, NodeToUiMessage};

pub enum CommandError {
    Transaction(UnmarshalError),
}

pub trait CommandContext {
    fn transact (&self, message: NodeFromUiMessage) -> Result<Option<NodeToUiMessage>, UnmarshalError>;
    fn console_out (&self, output: String);
    fn console_err (&self, output: String);
}

pub struct CommandContextReal {

}

impl CommandContext for CommandContextReal {
    fn transact(&self, message: NodeFromUiMessage) -> Result<Option<NodeToUiMessage>, UnmarshalError> {
        unimplemented!()
    }

    fn console_out(&self, output: String) {
        unimplemented!()
    }

    fn console_err(&self, output: String) {
        unimplemented!()
    }
}

impl CommandContextReal {
    fn new (port: u16) -> Self {
        Self {

        }
    }
}

pub trait Command: Debug {
    fn execute(&self, context: &Box<dyn CommandContext>) -> Result<(), CommandError>;
}

pub trait CommandProcessorFactory {
    fn make(&self, args: &Vec<String>) -> Box<dyn CommandProcessor>;
}

pub struct CommandProcessorFactoryReal {}

impl CommandProcessorFactory for CommandProcessorFactoryReal {
    fn make(&self, args: &Vec<String>) -> Box<dyn CommandProcessor> {
        unimplemented!()
    }
}

pub trait CommandProcessor {
    fn process (&self, command: Box<dyn Command>) -> Result<(), CommandError>;
}

pub struct CommandProcessorReal {

}

impl CommandProcessor for CommandProcessorReal {
    fn process(&self, command: Box<dyn Command>) -> Result<(), CommandError> {
        unimplemented!()
    }
}

impl CommandProcessorReal {
    pub fn new(args: &Vec<String>) -> Self {
        unimplemented!()
    }
}

pub struct CommandProcessorNull {}

impl CommandProcessor for CommandProcessorNull {
    fn process(&self, command: Box<dyn Command>) -> Result<(), CommandError> {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_factory::SetupCommand;

    #[test]
    #[should_panic(expected = "masq was not properly initialized")]
    fn null_command_processor_panics_properly() {
        let subject = CommandProcessorNull{};
        subject.process (Box::new (SetupCommand{values: vec![]}));
    }
}
