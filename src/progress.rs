//! Module that provides functions to display progress bars for compressing and decompressing files.
use std::{
    fmt::Arguments,
    io::{Read, Stderr, Write},
};

use indicatif::{ProgressBar, ProgressBarIter, ProgressStyle};

use crate::utils::colors::{RESET, YELLOW};

/// Draw a ProgressBar using a function that checks periodically for the progress
pub struct Progress {
    bar: ProgressBar,
}

pub trait OutputLine {
    fn output_line(&mut self, args: Arguments);
    fn output_line_info(&mut self, args: Arguments);
}

impl OutputLine for Progress {
    fn output_line(&mut self, args: Arguments) {
        self.bar.set_message(args.to_string());
    }

    fn output_line_info(&mut self, args: Arguments) {
        self.bar.set_message(format!("{}[INFO]{}{args}", *YELLOW, *RESET));
    }
}

impl OutputLine for Stderr {
    fn output_line(&mut self, args: Arguments) {
        self.write_fmt(args).unwrap();
    }

    fn output_line_info(&mut self, args: Arguments) {
        write!(self, "{}[INFO]{} {args}", *YELLOW, *RESET).unwrap();
        self.write_fmt(args).unwrap();
        self.write_all(b"\n").unwrap();
    }
}

impl<T: OutputLine + ?Sized> OutputLine for &mut T {
    fn output_line(&mut self, args: Arguments) {
        (*self).output_line(args)
    }

    fn output_line_info(&mut self, args: Arguments) {
        (*self).output_line_info(args);
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

        Progress { bar }
    }

    pub(crate) fn wrap_read<R: Read>(&self, read: R) -> ProgressBarIter<R> {
        self.bar.wrap_read(read)
    }

    pub(crate) fn wrap_write<W: Write>(&self, write: W) -> ProgressBarIter<W> {
        self.bar.wrap_write(write)
    }
}
