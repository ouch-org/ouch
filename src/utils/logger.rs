use std::sync::{mpsc, OnceLock};

pub use logger_thread::spawn_logger_thread;

use super::colors::{ORANGE, RESET, YELLOW};
use crate::accessible::is_running_in_accessible_mode;

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

pub fn warning(contents: String) {
    logger_thread::send_log_message(PrintMessage {
        contents,
        // Warnings are important and unlikely to flood, so they should be displayed
        accessible: true,
        level: MessageLevel::Warning,
    });
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
                    Some(format!("{}Warning:{} ", *ORANGE, *RESET))
                } else {
                    Some(format!("{}[WARNING]{} ", *ORANGE, *RESET))
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
    use std::sync::{Arc, Barrier};

    use super::*;

    type LogReceiver = mpsc::Receiver<Option<PrintMessage>>;
    type LogSender = mpsc::Sender<Option<PrintMessage>>;

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
        send_message(Some(msg));
    }

    #[track_caller]
    fn send_message(msg: Option<PrintMessage>) {
        get_sender().send(msg).expect("Failed to send internal message");
    }

    pub struct LoggerThreadHandle {
        shutdown_barrier: Arc<Barrier>,
    }

    impl LoggerThreadHandle {
        /// Tell logger to shutdown and waits till it does.
        pub fn shutdown_and_wait(self) {
            // Signal the shutdown
            send_message(None);
            // Wait for confirmation
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
        const BUFFER_CAPACITY: usize = 10;
        let mut buffer = Vec::<String>::with_capacity(BUFFER_CAPACITY);

        loop {
            let msg = log_receiver.recv().expect("Failed to receive log message");

            let is_shutdown_message = msg.is_none();

            // Append message to buffer
            if let Some(msg) = msg.as_ref().and_then(PrintMessage::to_processed_message) {
                buffer.push(msg);
            }

            let should_flush = buffer.len() == BUFFER_CAPACITY || is_shutdown_message;

            if should_flush {
                let text = buffer.join("\n");
                eprintln!("{text}");
                buffer.clear();
            }

            if is_shutdown_message {
                break;
            }
        }

        shutdown_barrier.wait();
    }
}
