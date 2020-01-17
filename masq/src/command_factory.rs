// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use masq_lib::command::{StdStreams};
use clap::ArgMatches;
use crate::command_processor::{Command, CommandContext, CommandError};

pub struct CommandFactory {

}

impl CommandFactory {
    pub fn new() -> Self {
        unimplemented!()
    }

    pub fn make(matches: ArgMatches) -> Box<dyn Command> {
        unimplemented!()
    }
}

struct SetupCommand {

}

impl Command for SetupCommand {
    fn execute(context: &mut CommandContext) -> Result<(), CommandError> {
        unimplemented!()
    }
}