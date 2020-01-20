// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use clap::ArgMatches;
use crate::commands::Command;

#[derive(Debug, PartialEq)]
pub enum CommandFactoryError {
    SyntaxError(String),
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
