// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use std::fmt::Debug;
use crate::command_context::{CommandContext};

#[derive (Debug, PartialEq)]
pub enum CommandError {
    Transaction(String),
}

pub trait Command: Debug {
    fn execute(&self, context: &mut dyn CommandContext) -> Result<(), CommandError>;
}

#[derive (Debug, PartialEq)]
pub struct SetupValue {
    name: String,
    value: String,
}

impl SetupValue {
    #[allow (dead_code)]
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
    fn execute(&self, _context: &mut dyn CommandContext) -> Result<(), CommandError> {
        unimplemented!()
    }
}
