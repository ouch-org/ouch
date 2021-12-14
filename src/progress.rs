//! Module that provides functions to display progress bars for compressing and decompressing files.
use std::{
    io,
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::Duration,
};

use indicatif::{ProgressBar, ProgressStyle};

/// Draw a ProgressBar using a function that checks periodically for the progress
pub struct Progress {
    draw_stop: Sender<()>,
    clean_done: Receiver<()>,
    display_handle: DisplayHandle,
}

/// Writes to this struct will be displayed on the progress bar or stdout depending on the
/// ProgressBarPolicy
struct DisplayHandle {
    buf: Vec<u8>,
    sender: Sender<String>,
}
impl io::Write for DisplayHandle {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend(buf);
        // Newline is the signal to flush
        if matches!(buf.last(), Some(&b'\n')) {
            self.buf.pop();
            self.flush()?;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        fn io_error<X>(_: X) -> io::Error {
            io::Error::new(io::ErrorKind::Other, "failed to flush buffer")
        }
        self.sender.send(String::from_utf8(self.buf.drain(..).collect()).map_err(io_error)?).map_err(io_error)
    }
}

impl Progress {
    /// Create a ProgressBar using a function that checks periodically for the progress
    /// If precise is true, the total_input_size will be displayed as the total_bytes size
    /// If ACCESSIBLE is set, this function returns None
    pub fn new_accessible_aware(
        total_input_size: u64,
        precise: bool,
        current_position_fn: Option<Box<dyn Fn() -> u64 + Send>>,
    ) -> Option<Self> {
        if *crate::cli::ACCESSIBLE.get().unwrap() {
            return None;
        }
        Some(Self::new(total_input_size, precise, current_position_fn))
    }

    fn new(total_input_size: u64, precise: bool, current_position_fn: Option<Box<dyn Fn() -> u64 + Send>>) -> Self {
        let (draw_tx, draw_rx) = mpsc::channel();
        let (clean_tx, clean_rx) = mpsc::channel();
        let (msg_tx, msg_rx) = mpsc::channel();

        thread::spawn(move || {
            let template = {
                let mut t = String::new();
                t += "{wide_msg} [{elapsed_precise}] ";
                if precise && current_position_fn.is_some() {
                    t += "[{bar:.cyan/blue}] ";
                } else {
                    t += "{spinner:.green} ";
                }
                if current_position_fn.is_some() {
                    t += "{bytes}/ ";
                }
                if precise {
                    t += "{total_bytes} ";
                }
                t += "({bytes_per_sec}, {eta}) {path}";
                t
            };
            let pb = ProgressBar::new(total_input_size);
            pb.set_style(ProgressStyle::default_bar().template(&template).progress_chars("#>-"));

            while draw_rx.try_recv().is_err() {
                if let Some(ref pos_fn) = current_position_fn {
                    pb.set_position(pos_fn());
                } else {
                    pb.tick();
                }
                if let Ok(msg) = msg_rx.try_recv() {
                    pb.set_message(msg);
                }
                thread::sleep(Duration::from_millis(100));
            }
            pb.finish();
            let _ = clean_tx.send(());
        });

        Progress {
            draw_stop: draw_tx,
            clean_done: clean_rx,
            display_handle: DisplayHandle { buf: Vec::new(), sender: msg_tx },
        }
    }

    pub(crate) fn display_handle(&mut self) -> &mut dyn io::Write {
        &mut self.display_handle
    }
}
impl Drop for Progress {
    fn drop(&mut self) {
        let _ = self.draw_stop.send(());
        let _ = self.clean_done.recv();
    }
}
