// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use clap::ArgMatches;
use crate::command_processor::{Command, CommandError, CommandContext};

#[derive(Debug, PartialEq)]
pub enum CommandFactoryError {

}

pub trait CommandFactory {
    fn make(&self, pieces: Vec<String>) -> Result<Box<dyn Command>, CommandFactoryError>;
}

pub struct CommandFactoryReal {

}

impl CommandFactory for CommandFactoryReal {
    fn make(&self, pieces: Vec<String>) -> Result<Box<dyn Command>, CommandFactoryError> {
        unimplemented!()
    }
}

impl CommandFactoryReal {
    pub fn new() -> Self {
        Self {

        }
    }
}

#[derive (Debug, PartialEq)]
pub struct SetupValue {
    name: String,
    value: String,
}

impl SetupValue {
    pub fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.to_string(),
            value: value.to_string(),
        }
    }
}

#[derive (Debug, PartialEq)]
pub struct SetupCommand {
    pub values: Vec<SetupValue>,
}

impl Command for SetupCommand {
    fn execute<'a>(&self, context: &mut Box<dyn CommandContext<'a> + 'a>) -> Result<(), CommandError> {
        unimplemented!()
    }
}