// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

mod command_factory;
mod command_processor;

use masq_lib::command;
use masq_lib::command::{StdStreams, Command};
use std::io;
use crate::command_factory::{CommandFactoryReal, CommandFactory};
use crate::command_processor::{CommandProcessor, CommandProcessorFactory, CommandProcessorFactoryReal};
use crate::command_factory::CommandFactoryError::SyntaxError;

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
        let mut processor = self.processor_factory.make (streams, args);
        let command_parts = match Self::extract_subcommand(args) {
            Ok(v) => v,
            Err(msg) => {
                writeln! (streams.stderr, "{}", msg).expect ("writeln! failed");
                return 1
            }
        };
        if let Err(msg) = self.handle_command(&mut processor, command_parts) {
            writeln! (streams.stderr, "{}", msg).expect ("writeln! failed");
            return 1
        }
        processor.shutdown();
        0
    }
}

impl Main {
    pub fn new() -> Self {
        Self {
            command_factory: Box::new(CommandFactoryReal::new()),
            processor_factory: Box::new (CommandProcessorFactoryReal{}),
        }
    }

    fn extract_subcommand(args: &[String]) -> Result<Vec<String>, String> {
        let mut args_vec: Vec<String> = args.into_iter().map(|s| s.clone()).collect();
        let mut subcommand_idx = 0;
        for idx in 1..args_vec.len() {
            let one = &args_vec[idx - 1];
            let two = &args_vec[idx];
            if !one.starts_with ("--") && !two.starts_with ("--") {
                return Ok(args_vec.into_iter ().skip (idx).collect())
            }
        }
        return Err(format!("No masq subcommand found in '{}'", args_vec.join(" ")));
    }

    fn handle_command(&self, processor: &mut Box<dyn CommandProcessor>, command_parts: Vec<String>) -> Result<(), String> {
        let command = match self.command_factory.make (command_parts) {
            Ok(c) => c,
            Err(SyntaxError(msg)) => return Err(msg),
        };
        if let Err(e) = processor.process (command) {
            return Err(format!("{:?}", e))
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, Arc};
    use masq_lib::test_utils::fake_stream_holder::{FakeStreamHolder};
    use crate::command_processor::{CommandError, CommandProcessorFactory, CommandContext};
    use std::cell::RefCell;
    use crate::command_factory::{CommandFactoryError};
    use masq_lib::ui_traffic_converter::{UnmarshalError, TrafficConversionError};
    use masq_lib::ui_gateway::{NodeToUiMessage, NodeFromUiMessage};
    use masq_lib::messages::{UiShutdownOrder, UiSetup};
    use lazy_static::lazy_static;
    use masq_lib::messages::ToMessageBody;
    use std::io::{Read, Write};
    use crate::command_processor::CommandError::Transaction;
    use masq_lib::ui_traffic_converter::TrafficConversionError::JsonSyntaxError;
    use masq_lib::ui_traffic_converter::UnmarshalError::Critical;

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

    struct CommandContextMock<'a> {
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
        fn new (streams: &'a mut StdStreams<'a>) -> Self {
            Self {
                transact_params: Arc::new(Mutex::new(vec![])),
                transact_results: vec![],
                streams,
            }
        }

        fn transact_params (mut self, params: &Arc<Mutex<Vec<NodeFromUiMessage>>>) -> Self {
            self.transact_params = params.clone();
            self
        }

        fn transact_result (mut self, result: Result<Option<NodeToUiMessage>, UnmarshalError>) -> Self {
            self.transact_results.push (result);
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
        process_params: Arc<Mutex<Vec<Box<dyn command_processor::Command>>>>,
        process_results: RefCell<Vec<Result<(), CommandError>>>,
        shutdown_params: Arc<Mutex<Vec<()>>>,
    }

    impl CommandProcessor for CommandProcessorMock {
        fn process(&mut self, command: Box<dyn command_processor::Command>) -> Result<(), CommandError> {
            self.process_params.lock().unwrap().push (command);
            self.process_results.borrow_mut().remove(0)
        }

        fn shutdown(&mut self) {
            self.shutdown_params.lock().unwrap().push (());
        }
    }

    impl CommandProcessorMock {
        fn new() -> Self {
            Self {
                process_params: Arc::new (Mutex::new (vec![])),
                process_results: RefCell::new (vec![]),
                shutdown_params: Arc::new (Mutex::new (vec![])),
            }
        }

        fn process_params (mut self, params: &Arc<Mutex<Vec<Box<dyn command_processor::Command>>>>) -> Self {
            self.process_params = params.clone();
            self
        }

        fn process_result (self, result: Result<(), CommandError>) -> Self {
            self.process_results.borrow_mut().push (result);
            self
        }

        fn shutdown_params (mut self, params: &Arc<Mutex<Vec<()>>>) -> Self {
            self.shutdown_params = params.clone();
            self
        }
    }

    struct CommandProcessorFactoryMock {
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
        fn execute<'a>(&self, context: &mut Box<dyn CommandContext<'a> + 'a>) -> Result<(), CommandError> {
            match context.transact (self.message.clone()) {
                Ok(_) => (),
                Err(e) => return Err(CommandError::Transaction(e)),
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

    #[test]
    fn go_works_when_everything_is_copacetic() {
        let command = MockCommand::new (ONE_WAY_MESSAGE.clone())
            .execute_result (Ok(()));
        let c_make_params_arc = Arc::new (Mutex::new (vec![]));
        let command_factory = CommandFactoryMock::new()
            .make_params(&c_make_params_arc)
            .make_result(Ok(Box::new (command)));
        let process_params_arc = Arc::new (Mutex::new (vec![]));
        let processor = CommandProcessorMock::new()
            .process_params (&process_params_arc)
            .process_result(Ok(()));
        let p_make_params_arc = Arc::new (Mutex::new(vec![]));
        let processor_factory = CommandProcessorFactoryMock::new()
            .make_params (&p_make_params_arc)
            .make_result (Box::new (processor));
        let mut subject = Main {
            command_factory: Box::new(command_factory),
            processor_factory: Box::new (processor_factory),
        };

        let result = subject.go(&mut FakeStreamHolder::new().streams(), &[
            "command".to_string(),
            "--param1".to_string(),
            "value1".to_string(),
            "--param2".to_string(),
            "value2".to_string(),
            "subcommand".to_string(),
            "--param3".to_string(),
            "value3".to_string(),
            "param4".to_string(),
            "param5".to_string(),
        ]);

        assert_eq! (result, 0);
        let c_make_params = c_make_params_arc.lock().unwrap();
        assert_eq! (*c_make_params, vec![
            vec!["subcommand".to_string(), "--param3".to_string(), "value3".to_string(),
                "param4".to_string(), "param5".to_string()],
        ]);
        let p_make_params = p_make_params_arc.lock().unwrap();
        assert_eq! (*p_make_params, vec![vec![
            "command".to_string(),
            "--param1".to_string(),
            "value1".to_string(),
            "--param2".to_string(),
            "value2".to_string(),
            "subcommand".to_string(),
            "--param3".to_string(),
            "value3".to_string(),
            "param4".to_string(),
            "param5".to_string(),
        ]]);
        let mut process_params = process_params_arc.lock().unwrap();
        let command = process_params.remove (0);
        let stream_holder_arc = Arc::new (Mutex::new (FakeStreamHolder::new()));
        let stream_holder_arc_inner = stream_holder_arc.clone();
        let transact_params_arc = Arc::new (Mutex::new (vec![]));
        let result = {
            let mut stream_holder = stream_holder_arc_inner.lock().unwrap();
            let mut streams = stream_holder.streams();
            let context = CommandContextMock::new(&mut streams)
                .transact_params(&transact_params_arc)
                .transact_result(Ok(None));
            let mut boxed_context: Box<dyn CommandContext> = Box::new (context);

            command.execute(&mut boxed_context)
        };

        assert_eq! (result, Ok(()));
        let transact_params = transact_params_arc.lock().unwrap();
        assert_eq! (*transact_params, vec![ONE_WAY_MESSAGE.clone()]);
        let stream_holder = stream_holder_arc.lock().unwrap();
        assert_eq! (stream_holder.stdout.get_string(), "MockCommand output\n".to_string());
        assert_eq! (stream_holder.stderr.get_string(), "MockCommand error\n".to_string());
    }

    #[test]
    fn go_works_when_given_no_subcommand() {
        let command = MockCommand::new (ONE_WAY_MESSAGE.clone())
            .execute_result (Ok(()));
        let command_factory = CommandFactoryMock::new();
        let processor = CommandProcessorMock::new();
        let processor_factory = CommandProcessorFactoryMock::new()
            .make_result (Box::new (processor));
        let mut subject = Main {
            command_factory: Box::new(command_factory),
            processor_factory: Box::new (processor_factory),
        };
        let mut stream_holder = FakeStreamHolder::new();

        let result = subject.go(&mut stream_holder.streams(), &[
            "command".to_string(),
            "--param1".to_string(),
            "value1".to_string(),
        ]);

        assert_eq! (result, 1);
        assert_eq! (stream_holder.stdout.get_string(), "".to_string());
        assert_eq! (stream_holder.stderr.get_string(), "No masq subcommand found in 'command --param1 value1'\n".to_string());
    }

    #[test]
    fn go_works_when_command_cant_be_created() {
        let command = MockCommand::new (ONE_WAY_MESSAGE.clone())
            .execute_result (Ok(()));
        let c_make_params_arc = Arc::new (Mutex::new (vec![]));
        let command_factory = CommandFactoryMock::new()
            .make_params(&c_make_params_arc)
            .make_result(Err(CommandFactoryError::SyntaxError("booga".to_string())));
        let processor = CommandProcessorMock::new();
        let processor_factory = CommandProcessorFactoryMock::new()
            .make_result (Box::new (processor));
        let mut subject = Main {
            command_factory: Box::new(command_factory),
            processor_factory: Box::new (processor_factory),
        };
        let mut stream_holder = FakeStreamHolder::new();

        let result = subject.go(&mut stream_holder.streams(), &[
            "command".to_string(),
            "subcommand".to_string(),
        ]);

        assert_eq! (result, 1);
        let c_make_params = c_make_params_arc.lock().unwrap();
        assert_eq! (*c_make_params, vec![
            vec!["subcommand".to_string()],
        ]);
        assert_eq! (stream_holder.stdout.get_string(), "".to_string());
        assert_eq! (stream_holder.stderr.get_string(), "booga\n".to_string());
    }

    #[test]
    fn go_works_when_command_is_unhappy() {
        let command = MockCommand::new (ONE_WAY_MESSAGE.clone())
            .execute_result (Ok(())); // irrelevant
        let command_factory = CommandFactoryMock::new()
            .make_result(Ok(Box::new (command)));
        let process_params_arc = Arc::new (Mutex::new (vec![]));
        let processor = CommandProcessorMock::new()
            .process_params (&process_params_arc)
            .process_result(Err(Transaction(Critical(JsonSyntaxError("booga".to_string())))));
        let processor_factory = CommandProcessorFactoryMock::new()
            .make_result (Box::new (processor));
        let mut subject = Main {
            command_factory: Box::new(command_factory),
            processor_factory: Box::new (processor_factory),
        };
        let mut stream_holder = FakeStreamHolder::new();

        let result = subject.go(&mut stream_holder.streams(), &[
            "command".to_string(),
            "subcommand".to_string(),
        ]);

        assert_eq! (result, 1);
        assert_eq! (stream_holder.stdout.get_string(), "".to_string());
        assert_eq! (stream_holder.stderr.get_string(), "Transaction(Critical(JsonSyntaxError(\"booga\")))\n".to_string());
    }
}
