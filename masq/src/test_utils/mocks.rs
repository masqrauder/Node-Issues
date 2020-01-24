// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use std::sync::{Mutex, Arc};
use std::cell::RefCell;
use crate::command_factory::{CommandFactoryError, CommandFactory};
use masq_lib::ui_traffic_converter::{UnmarshalError};
use masq_lib::ui_gateway::{NodeToUiMessage, NodeFromUiMessage};
use masq_lib::messages::{UiShutdownOrder, UiSetup};
use lazy_static::lazy_static;
use masq_lib::messages::ToMessageBody;
use std::io::{Read, Write};
use masq_lib::command::StdStreams;
use crate::commands::{CommandError, Command};
use crate::command_context::{CommandContext};
//use crate::command_context::{CommandContextFactory, CommandContextFactoryError};
use crate::commands::CommandError::Transaction;
use crate::command_processor::{CommandProcessor, CommandProcessorFactory};

lazy_static! {
    pub static ref ONE_WAY_MESSAGE: NodeFromUiMessage = NodeFromUiMessage {
        client_id: 0,
        body: UiShutdownOrder {}.tmb(0),
    };
    pub static ref TWO_WAY_MESSAGE: NodeFromUiMessage = NodeFromUiMessage {
        client_id: 0,
        body: UiSetup {values: vec![]}.tmb(0),
    };
}

//pub struct CommandContextFactoryMock<'a> {
//    make_params: Arc<Mutex<Vec<u16>>>,
//    make_results: RefCell<Vec<Result<Box<dyn CommandContext<'a>>, CommandContextFactoryError>>>,
//}
//
//impl CommandContextFactory for CommandContextFactoryMock<'_> {
//    fn make<'a>(&self, port: u16, streams: &StdStreams<'a>) -> Result<Box<dyn CommandContext<'a>>, CommandContextFactoryError> {
//        self.make_params.lock().unwrap().push (port);
//        self.make_results.borrow_mut().remove(0)
//    }
//}
//
//impl<'a> CommandContextFactoryMock<'a> {
//    pub fn new () -> Self {
//        Self {
//            make_params: Arc::new (Mutex::new (vec![])),
//            make_results: RefCell::new (vec![]),
//        }
//    }
//
//    pub fn make_params(mut self, params: &Arc<Mutex<Vec<u16>>>) -> Self {
//        self.make_params = params.clone();
//        self
//    }
//
//    pub fn make_result(self, result: Result<Box<dyn CommandContext<'a>>, CommandContextFactoryError>) -> Self {
//        self.make_results.borrow_mut().push (result);
//        self
//    }
//}

pub struct CommandContextMock<'a> {
    transact_params: Arc<Mutex<Vec<NodeFromUiMessage>>>,
    transact_results: Vec<Result<Option<NodeToUiMessage>, UnmarshalError>>,
    streams: &'a mut StdStreams<'a>,
}

impl<'a> CommandContext<'a> for CommandContextMock<'a> {
    fn transact(&mut self, message: NodeFromUiMessage) -> Result<Option<NodeToUiMessage>, UnmarshalError> {
        self.transact_params.lock().unwrap().push (message);
        self.transact_results.remove (0)
    }

    fn stdin(&mut self) -> &mut (dyn Read) {
        self.streams.stdin
    }

    fn stdout(&mut self) -> &mut (dyn Write) {
        self.streams.stdout
    }

    fn stderr(&mut self) -> &mut (dyn Write) {
        self.streams.stderr
    }
}

impl<'a> CommandContextMock<'a> {
    pub fn new (streams: &'a mut StdStreams<'a>) -> Self {
        Self {
            transact_params: Arc::new(Mutex::new(vec![])),
            transact_results: vec![],
            streams,
        }
    }

    pub fn transact_params (mut self, params: &Arc<Mutex<Vec<NodeFromUiMessage>>>) -> Self {
        self.transact_params = params.clone();
        self
    }

    pub fn transact_result (mut self, result: Result<Option<NodeToUiMessage>, UnmarshalError>) -> Self {
        self.transact_results.push (result);
        self
    }
}

pub struct CommandFactoryMock {
    make_params: Arc<Mutex<Vec<Vec<String>>>>,
    make_results: RefCell<Vec<Result<Box<dyn Command>, CommandFactoryError>>>,
}

impl CommandFactory for CommandFactoryMock {
    fn make(&self, pieces: Vec<String>) -> Result<Box<dyn Command>, CommandFactoryError> {
        self.make_params.lock().unwrap().push(pieces);
        self.make_results.borrow_mut().remove(0)
    }
}

impl CommandFactoryMock {
    pub fn new() -> Self {
        Self {
            make_params: Arc::new (Mutex::new (vec![])),
            make_results: RefCell::new (vec![]),
        }
    }

    pub fn make_params(mut self, params: &Arc<Mutex<Vec<Vec<String>>>>) -> Self {
        self.make_params = params.clone();
        self
    }

    pub fn make_result(self, result: Result<Box<dyn Command>, CommandFactoryError>) -> Self {
        self.make_results.borrow_mut().push (result);
        self
    }
}

pub struct CommandProcessorMock {
    process_params: Arc<Mutex<Vec<Box<dyn Command>>>>,
    process_results: RefCell<Vec<Result<(), CommandError>>>,
    shutdown_params: Arc<Mutex<Vec<()>>>,
}

impl CommandProcessor for CommandProcessorMock {
    fn process(&mut self, command: Box<dyn Command>) -> Result<(), CommandError> {
        self.process_params.lock().unwrap().push (command);
        self.process_results.borrow_mut().remove(0)
    }

    fn shutdown(&mut self) {
        self.shutdown_params.lock().unwrap().push (());
    }
}

impl CommandProcessorMock {
    pub fn new() -> Self {
        Self {
            process_params: Arc::new (Mutex::new (vec![])),
            process_results: RefCell::new (vec![]),
            shutdown_params: Arc::new (Mutex::new (vec![])),
        }
    }

    pub fn process_params (mut self, params: &Arc<Mutex<Vec<Box<dyn Command>>>>) -> Self {
        self.process_params = params.clone();
        self
    }

    pub fn process_result (self, result: Result<(), CommandError>) -> Self {
        self.process_results.borrow_mut().push (result);
        self
    }

    pub fn shutdown_params (mut self, params: &Arc<Mutex<Vec<()>>>) -> Self {
        self.shutdown_params = params.clone();
        self
    }
}

pub struct CommandProcessorFactoryMock {
    make_params: Arc<Mutex<Vec<Vec<String>>>>,
    make_results: RefCell<Vec<Box<dyn CommandProcessor>>>,
}

impl CommandProcessorFactory for CommandProcessorFactoryMock {
    fn make(&self, streams: &mut StdStreams<'_>, args: &[String]) -> Box<dyn CommandProcessor> {
        self.make_params.lock().unwrap().push (args.iter().map(|s| s.clone()).collect());
        self.make_results.borrow_mut().remove(0)
    }
}

impl CommandProcessorFactoryMock {
    pub fn new () -> Self {
        Self {
            make_params: Arc::new (Mutex::new (vec![])),
            make_results: RefCell::new (vec![]),
        }
    }

    pub fn make_params(mut self, params: &Arc<Mutex<Vec<Vec<String>>>>) -> Self {
        self.make_params = params.clone();
        self
    }

    pub fn make_result(self, result: Box<dyn CommandProcessor>) -> Self {
        self.make_results.borrow_mut().push (result);
        self
    }
}

pub struct MockCommand {
    message: NodeFromUiMessage,
    execute_results: RefCell<Vec<Result<(), CommandError>>>,
}

impl std::fmt::Debug for MockCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write! (f, "MockCommand")
    }
}

impl Command for MockCommand {
    fn execute<'a>(&self, context: &mut Box<dyn CommandContext<'a> + 'a>) -> Result<(), CommandError> {
        match context.transact (self.message.clone()) {
            Ok(_) => (),
            Err(e) => return Err(Transaction(e)),
        }
        writeln!(context.stdout(), "MockCommand output").unwrap();
        writeln!(context.stderr(), "MockCommand error").unwrap();
        self.execute_results.borrow_mut().remove (0)
    }
}

impl MockCommand {
    pub fn new (message: NodeFromUiMessage) -> Self {
        Self {
            message,
            execute_results: RefCell::new (vec![]),
        }
    }

    pub fn execute_result (self, result: Result<(), CommandError>) -> Self {
        self.execute_results.borrow_mut().push (result);
        self
    }
}
