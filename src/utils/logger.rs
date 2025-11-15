use std::{
    fmt,
    sync::{mpsc, Arc, Barrier, OnceLock},
    thread,
};

pub use logger_thread::spawn_logger_thread;

use super::colors::{GREEN, ORANGE, RESET};
use crate::accessible::is_running_in_accessible_mode;

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::utils::logger::info(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! info_accessible {
    ($($arg:tt)*) => {
        $crate::utils::logger::info_accessible(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! warning {
    ($($arg:tt)*) => {
        $crate::utils::logger::warning(format!($($arg)*))
    };
}

/// Global value used to determine which logs to display.
static LOG_DISPLAY_LEVEL: OnceLock<MessageLevel> = OnceLock::new();

fn should_display_log(level: &MessageLevel) -> bool {
    let global_level = &LOG_DISPLAY_LEVEL.get().copied().unwrap_or(MessageLevel::Info);
    level >= global_level
}

/// Set the value of the global [`LOG_DISPLAY_LEVEL`].
pub fn set_log_display_level(quiet: bool) {
    let level = if quiet { MessageLevel::Quiet } else { MessageLevel::Info };
    if LOG_DISPLAY_LEVEL.get().is_none() {
        LOG_DISPLAY_LEVEL.set(level).unwrap();
    }
}

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
    fn should_display(&self) -> bool {
        if self.level == MessageLevel::Quiet {
            return false;
        }

        if !should_display_log(&self.level) && !is_running_in_accessible_mode() {
            return false;
        }

        if self.level == MessageLevel::Info {
            return !is_running_in_accessible_mode() || self.accessible;
        }

        true
    }
}

impl fmt::Display for PrintMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        debug_assert!(
            self.should_display(),
            "Display called on message that shouldn't be displayed"
        );

        match self.level {
            MessageLevel::Info => {
                if !is_running_in_accessible_mode() {
                    write!(f, "{}[INFO]{} {}", *GREEN, *RESET, self.contents)?;
                } else if self.accessible {
                    write!(f, "{}Info:{} {}", *GREEN, *RESET, self.contents)?;
                }
            }
            MessageLevel::Warning => {
                if is_running_in_accessible_mode() {
                    write!(f, "{}Warning:{} {}", *ORANGE, *RESET, self.contents)?;
                } else {
                    write!(f, "{}[WARNING]{} {}", *ORANGE, *RESET, self.contents)?;
                }
            }
            MessageLevel::Quiet => {}
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
enum MessageLevel {
    Info,
    Warning,
    Quiet,
}

mod logger_thread {
    use std::{
        io::{stderr, Write},
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

        let mut buffer = Vec::new();

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
                    if msg.should_display() {
                        writeln!(buffer, "{msg}").unwrap();
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

    fn flush_logs_to_stderr(buffer: &mut Vec<u8>) {
        if !buffer.is_empty() {
            if let Err(err) = stderr().write_all(buffer) {
                panic!("Failed to write to STDERR: {err}");
            }
            buffer.clear();
        }
    }
}
