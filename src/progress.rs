//! Module that provides functions to display progress bars for compressing and decompressing files.
use std::{
    io::{self, Read, Write},
    mem,
};

use indicatif::{ProgressBar, ProgressBarIter, ProgressStyle};

/// Draw a ProgressBar using a function that checks periodically for the progress
pub struct Progress {
    bar: ProgressBar,
    buf: Vec<u8>,
}

impl Write for Progress {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend(buf);

        if self.buf.last() == Some(&b'\n') {
            self.buf.pop();
            self.flush()?;
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.bar.set_message(
            String::from_utf8(mem::take(&mut self.buf))
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "Failed to parse buffer content as utf8"))?,
        );
        Ok(())
    }
}

impl Progress {
    pub(crate) fn new(total_input_size: u64, precise: bool, position_updates: bool) -> Self {
        let template = {
            let mut t = String::new();
            t += "{wide_msg} [{elapsed_precise}] ";
            if precise && position_updates {
                t += "[{bar:.cyan/blue}] ";
            } else {
                t += "{spinner:.green} ";
            }
            if position_updates {
                t += "{bytes}/ ";
            }
            if precise {
                t += "{total_bytes} ";
            }
            t += "({bytes_per_sec}, {eta}) {path}";
            t
        };
        let bar = ProgressBar::new(total_input_size)
            .with_style(ProgressStyle::with_template(&template).unwrap().progress_chars("#>-"));

        Progress { bar, buf: Vec::new() }
    }

    pub(crate) fn wrap_read<R: Read>(&self, read: R) -> ProgressBarIter<R> {
        self.bar.wrap_read(read)
    }

    pub(crate) fn wrap_write<W: Write>(&self, write: W) -> ProgressBarIter<W> {
        self.bar.wrap_write(write)
    }
}
