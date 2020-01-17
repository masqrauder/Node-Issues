// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

pub struct CommandContext {

}

pub enum CommandError {

}

pub trait Command {
    fn execute(context: &mut CommandContext) -> Result<(), CommandError>;
}

pub trait CommandProcessor {
    fn process (command: Box<dyn Command>) -> Result<(), CommandError>;
}

pub struct CommandProcessorReal {

}

impl CommandProcessor for CommandProcessorReal {
    fn process(command: Box<dyn Command>) -> Result<(), CommandError> {
        unimplemented!()
    }
}

impl CommandProcessorReal {
    pub fn new(args: &Vec<String>) -> Self {
        unimplemented!()
    }
}

