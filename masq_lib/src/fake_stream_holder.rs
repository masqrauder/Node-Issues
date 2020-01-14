// Copyright (c) 2017-2019, Substratum LLC (https://substratum.net) and/or its affiliates. All rights reserved.

use crate::command::StdStreams;
use std::cmp::min;
use std::io;
use std::io::Error;
use std::io::Read;
use std::io::Write;

pub struct ByteArrayWriter {
    pub byte_array: Vec<u8>,
    pub next_error: Option<Error>,
}

impl Default for ByteArrayWriter {
    fn default() -> Self {
        let vec = Vec::new();
        ByteArrayWriter {
            byte_array: vec,
            next_error: None,
        }
    }
}

impl ByteArrayWriter {
    pub fn new() -> ByteArrayWriter {
        Self::default()
    }

    pub fn get_bytes(&self) -> &[u8] {
        self.byte_array.as_slice()
    }
    pub fn get_string(&self) -> String {
        String::from_utf8(self.byte_array.clone()).unwrap()
    }

    pub fn reject_next_write(&mut self, error: Error) {
        self.next_error = Some(error);
    }
}

impl Write for ByteArrayWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Some(next_error) = self.next_error.take() {
            Err(next_error)
        }
        else {
            for byte in buf {
                self.byte_array.push(*byte)
            }
            Ok(buf.len())
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub struct ByteArrayReader {
    byte_array: Vec<u8>,
    position: usize,
}

impl ByteArrayReader {
    pub fn new(byte_array: &[u8]) -> ByteArrayReader {
        ByteArrayReader {
            byte_array: byte_array.to_vec(),
            position: 0,
        }
    }
}

impl Read for ByteArrayReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let to_copy = min(buf.len(), self.byte_array.len() - self.position);
        #[allow (clippy::needless_range_loop)]
        for idx in 0..to_copy {
            buf[idx] = self.byte_array[self.position + idx]
        }
        self.position += to_copy;
        Ok(to_copy)
    }
}

pub struct FakeStreamHolder {
    pub stdin: ByteArrayReader,
    pub stdout: ByteArrayWriter,
    pub stderr: ByteArrayWriter,
}

impl Default for FakeStreamHolder {
    fn default() -> Self {
        FakeStreamHolder {
            stdin: ByteArrayReader::new(&[0; 0]),
            stdout: ByteArrayWriter::new(),
            stderr: ByteArrayWriter::new(),
        }
    }
}

impl FakeStreamHolder {
    pub fn new() -> FakeStreamHolder {
        Self::default()
    }

    pub fn streams(&mut self) -> StdStreams<'_> {
        StdStreams {
            stdin: &mut self.stdin,
            stdout: &mut self.stdout,
            stderr: &mut self.stderr,
        }
    }
}
