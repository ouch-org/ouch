use std::sync::{mpsc, Arc, Barrier, OnceLock};

pub use logger_thread::spawn_logger_thread;

use super::colors::{ORANGE, RESET, YELLOW};
use crate::accessible::is_running_in_accessible_mode;

/// Asks logger to flush all messages, useful before starting STDIN interaction.
#[track_caller]
pub fn flush_messages() {
    logger_thread::send_flush_message_and_wait();
}

/// An `[INFO]` log to be displayed if we're not running accessibility mode.
///
/// Same as `.info_accessible()`, but only displayed if accessibility mode
/// is turned off, which is detected by the function
/// `is_running_in_accessible_mode`.
///
/// Read more about accessibility mode in `accessible.rs`.
#[track_caller]
pub fn info(contents: String) {
    info_with_accessibility(contents, false);
}

/// An `[INFO]` log to be displayed.
///
/// Same as `.info()`, but also displays if `is_running_in_accessible_mode`
/// returns `true`.
///
/// Read more about accessibility mode in `accessible.rs`.
#[track_caller]
pub fn info_accessible(contents: String) {
    info_with_accessibility(contents, true);
}

#[track_caller]
fn info_with_accessibility(contents: String, accessible: bool) {
    logger_thread::send_log_message(PrintMessage {
        contents,
        accessible,
        level: MessageLevel::Info,
    });
}

#[track_caller]
pub fn warning(contents: String) {
    logger_thread::send_log_message(PrintMessage {
        contents,
        // Warnings are important and unlikely to flood, so they should be displayed
        accessible: true,
        level: MessageLevel::Warning,
    });
}

#[derive(Debug)]
enum Message {
    Flush { finished_barrier: Arc<Barrier> },
    FlushAndShutdown,
    PrintMessage(PrintMessage),
}

/// Message object used for sending logs from worker threads to a logging thread via channels.
/// See <https://github.com/ouch-org/ouch/issues/643>
#[derive(Debug)]
struct PrintMessage {
    contents: String,
    accessible: bool,
    level: MessageLevel,
}

impl PrintMessage {
    fn to_processed_message(&self) -> Option<String> {
        match self.level {
            MessageLevel::Info => {
                if self.accessible {
                    if is_running_in_accessible_mode() {
                        Some(format!("{}Info:{} {}", *YELLOW, *RESET, self.contents))
                    } else {
                        Some(format!("{}[INFO]{} {}", *YELLOW, *RESET, self.contents))
                    }
                } else if !is_running_in_accessible_mode() {
                    Some(format!("{}[INFO]{} {}", *YELLOW, *RESET, self.contents))
                } else {
                    None
                }
            }
            MessageLevel::Warning => {
                if is_running_in_accessible_mode() {
                    Some(format!("{}Warning:{} {}", *ORANGE, *RESET, self.contents))
                } else {
                    Some(format!("{}[WARNING]{} {}", *ORANGE, *RESET, self.contents))
                }
            }
        }
    }
}

#[derive(Debug, PartialEq)]
enum MessageLevel {
    Info,
    Warning,
}

mod logger_thread {
    use std::{
        sync::{mpsc::RecvTimeoutError, Arc, Barrier},
        time::Duration,
    };

    use super::*;

    type LogReceiver = mpsc::Receiver<Message>;
    type LogSender = mpsc::Sender<Message>;

    static SENDER: OnceLock<LogSender> = OnceLock::new();

    #[track_caller]
    fn setup_channel() -> LogReceiver {
        let (tx, rx) = mpsc::channel();
        SENDER.set(tx).expect("`setup_channel` should only be called once");
        rx
    }

    #[track_caller]
    fn get_sender() -> &'static LogSender {
        SENDER.get().expect("No sender, you need to call `setup_channel` first")
    }

    #[track_caller]
    pub(super) fn send_log_message(msg: PrintMessage) {
        get_sender()
            .send(Message::PrintMessage(msg))
            .expect("Failed to send print message");
    }

    #[track_caller]
    fn send_shutdown_message() {
        get_sender()
            .send(Message::FlushAndShutdown)
            .expect("Failed to send shutdown message");
    }

    #[track_caller]
    pub(super) fn send_flush_message_and_wait() {
        let barrier = Arc::new(Barrier::new(2));

        get_sender()
            .send(Message::Flush {
                finished_barrier: barrier.clone(),
            })
            .expect("Failed to send shutdown message");

        barrier.wait();
    }

    pub struct LoggerThreadHandle {
        shutdown_barrier: Arc<Barrier>,
    }

    impl LoggerThreadHandle {
        /// Tell logger to shutdown and waits till it does.
        pub fn shutdown_and_wait(self) {
            // Signal the shutdown
            send_shutdown_message();
            // Wait for confirmation
            self.shutdown_barrier.wait();
        }
    }

    #[cfg(test)]
    // shutdown_and_wait must be called manually, but to keep 'em clean, in
    // case of tests just do it on drop
    impl Drop for LoggerThreadHandle {
        fn drop(&mut self) {
            send_shutdown_message();
            self.shutdown_barrier.wait();
        }
    }

    pub fn spawn_logger_thread() -> LoggerThreadHandle {
        let log_receiver = setup_channel();

        let shutdown_barrier = Arc::new(Barrier::new(2));

        let handle = LoggerThreadHandle {
            shutdown_barrier: shutdown_barrier.clone(),
        };

        rayon::spawn(move || run_logger(log_receiver, shutdown_barrier));

        handle
    }

    fn run_logger(log_receiver: LogReceiver, shutdown_barrier: Arc<Barrier>) {
        const FLUSH_TIMEOUT: Duration = Duration::from_millis(200);

        let mut buffer = Vec::<String>::with_capacity(16);

        loop {
            let msg = match log_receiver.recv_timeout(FLUSH_TIMEOUT) {
                Ok(msg) => msg,
                Err(RecvTimeoutError::Timeout) => {
                    flush_logs_to_stderr(&mut buffer);
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => unreachable!("sender is static"),
            };

            match msg {
                Message::PrintMessage(msg) => {
                    // Append message to buffer
                    if let Some(msg) = msg.to_processed_message() {
                        buffer.push(msg);
                    }

                    if buffer.len() == buffer.capacity() {
                        flush_logs_to_stderr(&mut buffer);
                    }
                }
                Message::FlushAndShutdown => {
                    flush_logs_to_stderr(&mut buffer);
                    break;
                }
                Message::Flush { finished_barrier } => {
                    flush_logs_to_stderr(&mut buffer);
                    finished_barrier.wait();
                    break;
                }
            }
        }

        shutdown_barrier.wait();
    }

    fn flush_logs_to_stderr(buffer: &mut Vec<String>) {
        if !buffer.is_empty() {
            let text = buffer.join("\n");
            eprintln!("{text}");
            buffer.clear();
        }
    }
}
