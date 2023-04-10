use std::{io};
use std::collections::VecDeque;

use tracing_subscriber::fmt::MakeWriter;

#[derive(Default)]
pub(crate) struct Log {
}

#[derive(Default)]
pub(crate) struct LogWriter {
    buf: VecDeque<String>,
}

impl io::Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        println!("got: {}", buf.len());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        println!("flush");
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Log {
    type Writer = LogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LogWriter {
            buf: Vec::new(),
        }
    }
}
