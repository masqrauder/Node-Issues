// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

mod command_factory;
mod command_processor;

use masq_lib::command::{Command, StdStreams};
use std::io;
use crate::command_factory::CommandFactory;
use crate::command_processor::{CommandProcessor, CommandProcessorReal};

fn main() {
    let mut streams: StdStreams<'_> = StdStreams {
        stdin: &mut io::stdin(),
        stdout: &mut io::stdout(),
        stderr: &mut io::stderr(),
    };

    let args: Vec<String> = std::env::args().collect();
    let mut command = Main {
        factory: CommandFactory::new(),
        processor: Box::new(CommandProcessorReal::new(&args))
    };
    let streams_ref: &mut StdStreams<'_> = &mut streams;
    let exit_code = command.go(streams_ref, &args);
    ::std::process::exit(i32::from(exit_code));
}

struct Main {
    factory: CommandFactory,
    processor: Box<dyn CommandProcessor>,
}

impl Command for Main {
    fn go(&mut self, streams: &mut StdStreams<'_>, args: &[String]) -> u8 {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, Arc};
    use masq_lib::fake_stream_holder::FakeStreamHolder;

    #[test]
    fn successful_setup_is_processed() {
        let process_params_arc = Arc::new(Mutex::new(vec![]));
        let processor = CommandProcessorMock::new()
            .process_params(&process_params_arc);
        let mut subject = Main {
            factory: CommandFactory::new(),
            processor: Box::new (processor),
        };
        let mut streams = FakeStreamHolder::new();

        let exit_code = subject.go (&mut streams.streams(), &vec![]);

        assert_eq! (exit_code, 0);
        let process_params = process_params_arc.lock().unwrap();
        assert_eq! (*process_params, )
        unimplemented!()
    }
}
