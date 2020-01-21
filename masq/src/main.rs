// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

mod command_context;
mod command_factory;
pub mod command_processor;
mod commands;

#[cfg(test)]
mod test_utils;

use masq_lib::command;
use masq_lib::command::{StdStreams, Command};
use std::io;
use crate::command_factory::{CommandFactoryReal, CommandFactory};
use crate::command_processor::{CommandProcessor, CommandProcessorFactory, CommandProcessorFactoryReal};
use crate::command_factory::CommandFactoryError::SyntaxError;
//use crate::command_context::CommandContextFactoryReal;

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
            processor_factory: Box::new (CommandProcessorFactoryReal{
//                command_context_factory: Box::new(CommandContextFactoryReal{})
            }),
        }
    }

    fn extract_subcommand(args: &[String]) -> Result<Vec<String>, String> {
        let mut args_vec: Vec<String> = args.into_iter().map(|s| s.clone()).collect();
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
    use crate::test_utils::mocks::{MockCommand, CommandFactoryMock, CommandProcessorMock, CommandProcessorFactoryMock, CommandContextMock};
    use std::sync::{Arc, Mutex};
    use masq_lib::test_utils::fake_stream_holder::FakeStreamHolder;
    use masq_lib::ui_traffic_converter::TrafficConversionError::JsonSyntaxError;
    use masq_lib::ui_traffic_converter::UnmarshalError::Critical;
    use crate::test_utils::mocks::ONE_WAY_MESSAGE;
    use crate::commands::CommandError::Transaction;
    use crate::command_context::CommandContext;

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
            .make_result(Err(SyntaxError("booga".to_string())));
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
