use std::{
    sync::{mpsc, Arc, Barrier, OnceLock},
    thread,
};

pub use logger_thread::spawn_logger_thread;

use super::colors::{ORANGE, RESET, YELLOW};
use crate::accessible::is_running_in_accessible_mode;

/// Asks logger to shutdown and waits till it flushes all pending messages.
#[track_caller]
pub fn shutdown_logger_and_wait() {
    logger_thread::send_shutdown_command_and_wait();
}

/// Asks logger to flush all messages, useful before starting STDIN interaction.
#[track_caller]
pub fn flush_messages() {
    logger_thread::send_flush_command_and_wait();
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
    logger_thread::send_print_command(PrintMessage {
        contents,
        accessible,
        level: MessageLevel::Info,
    });
}

#[track_caller]
pub fn warning(contents: String) {
    logger_thread::send_print_command(PrintMessage {
        contents,
        // Warnings are important and unlikely to flood, so they should be displayed
        accessible: true,
        level: MessageLevel::Warning,
    });
}

#[derive(Debug)]
enum LoggerCommand {
    Print(PrintMessage),
    Flush { finished_barrier: Arc<Barrier> },
    FlushAndShutdown { finished_barrier: Arc<Barrier> },
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
    fn to_formatted_message(&self) -> Option<String> {
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

    type LogReceiver = mpsc::Receiver<LoggerCommand>;
    type LogSender = mpsc::Sender<LoggerCommand>;

    static SENDER: OnceLock<LogSender> = OnceLock::new();

    #[track_caller]
    fn setup_channel() -> Option<LogReceiver> {
        let mut optional = None;
        SENDER.get_or_init(|| {
            let (tx, rx) = mpsc::channel();
            optional = Some(rx);
            tx
        });
        optional
    }

    #[track_caller]
    fn get_sender() -> &'static LogSender {
        SENDER.get().expect("No sender, you need to call `setup_channel` first")
    }

    #[track_caller]
    pub(super) fn send_print_command(msg: PrintMessage) {
        if cfg!(test) {
            spawn_logger_thread();
        }
        get_sender()
            .send(LoggerCommand::Print(msg))
            .expect("Failed to send print command");
    }

    #[track_caller]
    pub(super) fn send_flush_command_and_wait() {
        let barrier = Arc::new(Barrier::new(2));

        get_sender()
            .send(LoggerCommand::Flush {
                finished_barrier: barrier.clone(),
            })
            .expect("Failed to send flush command");

        barrier.wait();
    }

    #[track_caller]
    pub(super) fn send_shutdown_command_and_wait() {
        let barrier = Arc::new(Barrier::new(2));

        get_sender()
            .send(LoggerCommand::FlushAndShutdown {
                finished_barrier: barrier.clone(),
            })
            .expect("Failed to send shutdown command");

        barrier.wait();
    }

    pub fn spawn_logger_thread() {
        if let Some(log_receiver) = setup_channel() {
            thread::spawn(move || run_logger(log_receiver));
        }
    }

    fn run_logger(log_receiver: LogReceiver) {
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
                LoggerCommand::Print(msg) => {
                    // Append message to buffer
                    if let Some(msg) = msg.to_formatted_message() {
                        buffer.push(msg);
                    }

                    if buffer.len() == buffer.capacity() {
                        flush_logs_to_stderr(&mut buffer);
                    }
                }
                LoggerCommand::Flush { finished_barrier } => {
                    flush_logs_to_stderr(&mut buffer);
                    finished_barrier.wait();
                }
                LoggerCommand::FlushAndShutdown { finished_barrier } => {
                    flush_logs_to_stderr(&mut buffer);
                    finished_barrier.wait();
                    return;
                }
            }
        }
    }

    fn flush_logs_to_stderr(buffer: &mut Vec<String>) {
        if !buffer.is_empty() {
            let text = buffer.join("\n");
            eprintln!("{text}");
            buffer.clear();
        }
    }
}
