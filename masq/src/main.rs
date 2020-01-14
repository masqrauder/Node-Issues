// Copyright (c) 2019-2020, MASQ (https://masq.ai) and/or its affiliates. All rights reserved.

use masq_lib::command::{Command, StdStreams};
use std::io;

fn main() {
    let mut streams: StdStreams<'_> = StdStreams {
        stdin: &mut io::stdin(),
        stdout: &mut io::stdout(),
        stderr: &mut io::stderr(),
    };

    let mut command = Main{};
    let streams_ref: &mut StdStreams<'_> = &mut streams;
    let args: Vec<String> = std::env::args().collect();
    let exit_code = command.go(streams_ref, &args);
    ::std::process::exit(i32::from(exit_code));
}

struct Main {}

impl Command for Main {
    fn go(&mut self, streams: &mut StdStreams<'_>, args: &[String]) -> u8 {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn nothing() {
        unimplemented!()
    }
}
