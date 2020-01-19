// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

mod command_factory;
mod command_processor;

use masq_lib::command;
use masq_lib::command::{StdStreams, Command};
use std::io;
use crate::command_factory::{CommandFactoryReal, CommandFactory};
use crate::command_processor::{CommandProcessor, CommandProcessorFactory, CommandProcessorFactoryReal};

fn main() {
    let mut streams: StdStreams<'_> = StdStreams {
        stdin: &mut io::stdin(),
        stdout: &mut io::stdout(),
        stderr: &mut io::stderr(),
    };

    let args: Vec<String> = std::env::args().collect();
    let streams_ref: &mut StdStreams<'_> = &mut streams;
    let exit_code = Main::new().go(streams_ref, &args);
    ::std::process::exit(i32::from(exit_code));
}

struct Main {
    command_factory: Box<dyn CommandFactory>,
    processor_factory: Box<dyn CommandProcessorFactory>,
}

impl command::Command for Main {
    fn go(&mut self, streams: &mut StdStreams<'_>, args: &[String]) -> u8 {
        let args_vec = args.iter().map(|s| s.clone()).collect();
        let processor = self.processor_factory.make (&args_vec);
        let command = self.command_factory.make (args_vec.into_iter().skip(1).collect());

        // initialize, replace CommandProcessor
        // Tear off first parameter (executed command), make a command with the factory, and send it to the CommandProcessor
        // Shut down the CommandProcessor (or the CommandContext, whichever makes more sense)
        1
    }
}

impl Main {
    pub fn new() -> Self {
        Self {
            command_factory: Box::new(CommandFactoryReal::new()),
            processor_factory: Box::new (CommandProcessorFactoryReal{}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, Arc};
    use masq_lib::test_utils::fake_stream_holder::FakeStreamHolder;
    use crate::command_processor::{CommandError, CommandProcessorFactory, CommandContextReal, CommandContext};
    use std::cell::RefCell;
    use crate::command_factory::{CommandFactoryError};
    use masq_lib::ui_traffic_converter::UnmarshalError;
    use masq_lib::ui_gateway::{NodeToUiMessage, NodeFromUiMessage};
    use masq_lib::messages::{UiShutdownOrder, UiSetup};
    use lazy_static::lazy_static;
    use masq_lib::messages::ToMessageBody;

    lazy_static! {
        static ref ONE_WAY_MESSAGE: NodeFromUiMessage = NodeFromUiMessage {
            client_id: 0,
            body: UiShutdownOrder {}.tmb(0),
        };
        static ref TWO_WAY_MESSAGE: NodeFromUiMessage = NodeFromUiMessage {
            client_id: 0,
            body: UiSetup {values: vec![]}.tmb(0),
        };
    }

    struct CommandContextMock {
        transact_params: Arc<Mutex<Vec<NodeFromUiMessage>>>,
        transact_results: RefCell<Vec<Result<Option<NodeToUiMessage>, UnmarshalError>>>,
    }

    impl CommandContext for CommandContextMock {
        fn transact(&self, message: NodeFromUiMessage) -> Result<Option<NodeToUiMessage>, UnmarshalError> {
            self.transact_params.lock().unwrap().push (message);
            self.transact_results.borrow_mut().remove (0)
        }

        fn console_out(&self, output: String) {
            unimplemented!()
        }

        fn console_err(&self, output: String) {
            unimplemented!()
        }
    }

    impl CommandContextMock {
        fn new () -> Self {
            Self {
                transact_params: Arc::new(Mutex::new(vec![])),
                transact_results: RefCell::new(vec![])
            }
        }

        fn transact_params (mut self, params: &Arc<Mutex<Vec<NodeFromUiMessage>>>) -> Self {
            self.transact_params = params.clone();
            self
        }

        fn transact_result (self, result: Result<Option<NodeToUiMessage>, UnmarshalError>) -> Self {
            self.transact_results.borrow_mut().push (result);
            self
        }
    }

    struct CommandFactoryMock {
        make_params: Arc<Mutex<Vec<Vec<String>>>>,
        make_results: RefCell<Vec<Result<Box<dyn command_processor::Command>, CommandFactoryError>>>,
    }

    impl CommandFactory for CommandFactoryMock {
        fn make(&self, pieces: Vec<String>) -> Result<Box<dyn command_processor::Command>, CommandFactoryError> {
            self.make_params.lock().unwrap().push(pieces);
            self.make_results.borrow_mut().remove(0)
        }
    }

    impl CommandFactoryMock {
        fn new() -> Self {
            Self {
                make_params: Arc::new (Mutex::new (vec![])),
                make_results: RefCell::new (vec![]),
            }
        }

        fn make_params(mut self, params: &Arc<Mutex<Vec<Vec<String>>>>) -> Self {
            self.make_params = params.clone();
            self
        }

        fn make_result(self, result: Result<Box<dyn command_processor::Command>, CommandFactoryError>) -> Self {
            self.make_results.borrow_mut().push (result);
            self
        }
    }

    struct CommandProcessorMock {
        context: Box<dyn CommandContext>,
    }

    impl CommandProcessor for CommandProcessorMock {
        fn process(&self, command: Box<dyn command_processor::Command>) -> Result<(), CommandError> {
            command.execute (&self.context)
        }
    }

    impl CommandProcessorMock {
        fn new(context: CommandContextMock) -> Self {
            Self {
                context: Box::new (context),
            }
        }
    }

    struct CommandProcessorFactoryMock {
        make_result: RefCell<Option<CommandProcessorMock>>,
        make_params: Arc<Mutex<Vec<String>>>,
    }

    impl CommandProcessorFactory for CommandProcessorFactoryMock {
        fn make(&self, args: &Vec<String>) -> Box<dyn CommandProcessor> {
            let mut args_ref = self.make_params.lock().unwrap();
            args_ref.clear();
            args_ref.extend (args.clone());
            Box::new (self.make_result.borrow_mut().take().unwrap())
        }
    }

    impl CommandProcessorFactoryMock {
        pub fn new (processor: CommandProcessorMock) -> Self {
            Self {
                make_result: RefCell::new (Some(processor)),
                make_params: Arc::new (Mutex::new (vec![])),
            }
        }

        pub fn make_params(mut self, params: &Arc<Mutex<Vec<String>>>) -> Self {
            self.make_params = params.clone();
            self
        }
    }

    struct MockCommand {
        message: NodeFromUiMessage,
        execute_results: RefCell<Vec<Result<(), CommandError>>>,
    }

    impl std::fmt::Debug for MockCommand {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
            write! (f, "MockCommand")
        }
    }

    impl command_processor::Command for MockCommand {
        fn execute(&self, context: &Box<dyn CommandContext>) -> Result<(), CommandError> {
            match context.transact (self.message.clone()) {
                Ok(_) => (),
                Err(e) => return Err(CommandError::Transaction(e)),
            }
            context.console_out (format!("MockCommand output"));
            context.console_err (format!("MockCommand error"));
            Ok(())
        }
    }

    impl MockCommand {
        pub fn new (message: NodeFromUiMessage) -> Self {
            Self {
                message,
                execute_results: RefCell::new (vec![]),
            }
        }
    }

    #[test]
    fn successful_setup_is_processed() {
        let expected_command = MockCommand::new(ONE_WAY_MESSAGE.clone());
        let make_params_arc = Arc::new (Mutex::new (vec![]));
        let command_factory = CommandFactoryMock::new()
            .make_params(&make_params_arc)
            .make_result(Ok(Box::new (expected_command)));
        let transact_params_arc = Arc::new (Mutex::new (vec![]));
        let context = CommandContextMock::new()
            .transact_params(&transact_params_arc)
            .transact_result(Ok(None));
        let processor = CommandProcessorMock::new(context);
        let make_params_arc = Arc::new (Mutex::new(vec![]));
        let processor_factory = CommandProcessorFactoryMock::new(processor)
            .make_params (&make_params_arc);
        let mut subject = Main {
            command_factory: Box::new(command_factory),
            processor_factory: Box::new (processor_factory),
        };
        let mut streams = FakeStreamHolder::new();
        let args = vec![
            "".to_string(),
            "--ui-port".to_string(), "12345".to_string(),
            "setup".to_string(),
            "name=value".to_string(),
            "configure=me".to_string(),
        ];

        let exit_code = subject.go (&mut streams.streams(), &args);

        assert_eq! (exit_code, 0);
        let mut transact_params = transact_params_arc.lock().unwrap();
        unimplemented!()
    }
}
