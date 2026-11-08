use std::{
    fmt,
    sync::{Arc, Barrier, OnceLock, mpsc},
    thread,
};

pub use logger_thread::spawn_logger_thread;

use super::{
    colors::{GREEN, ORANGE, RESET},
    formatting::{contains_unsafe_display_char, sanitize_for_display},
    progress::{self, ProgressCommand, ProgressState},
};
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

/// Sends a progress command to the logging thread, which owns the progress line.
///
/// The terminal is only ever written by the logging thread, so the indicator
/// cannot interleave with log lines.
pub(crate) fn send_progress_command(command: ProgressCommand) {
    logger_thread::send_progress_command(command);
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
    Progress(ProgressCommand),
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
        io::{Write, stderr},
        sync::{Arc, Barrier, mpsc::RecvTimeoutError},
        time::{Duration, Instant},
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

    /// Progress commands are fire-and-forget: a task may outlive the logging
    /// thread during shutdown, so failures are ignored instead of panicking.
    pub(super) fn send_progress_command(command: ProgressCommand) {
        if cfg!(test) {
            spawn_logger_thread();
        }
        let _ = get_sender().send(LoggerCommand::Progress(command));
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
        // Wake up often enough to animate the progress line, and to flush
        // messages that were not followed by a flush command.
        const TICK: Duration = Duration::from_millis(100);
        // Redrawing more often than this only burns terminal bandwidth.
        const REDRAW_INTERVAL: Duration = Duration::from_millis(100);

        let mut buffer = Vec::new();
        let mut progress = ProgressState::default();
        // Width of the progress line currently on screen, for erasing it.
        let mut drawn_width = 0;
        // `None` means nothing was drawn yet, so the first line is drawn at once.
        let mut last_redraw: Option<Instant> = None;

        loop {
            let msg = match log_receiver.recv_timeout(TICK) {
                Ok(msg) => msg,
                Err(RecvTimeoutError::Timeout) => {
                    flush_logs_to_stderr(&mut buffer);
                    redraw_progress_line(&progress, &mut drawn_width, &mut last_redraw, REDRAW_INTERVAL);
                    continue;
                }
                Err(RecvTimeoutError::Disconnected) => unreachable!("sender is static"),
            };

            match msg {
                LoggerCommand::Print(msg) => {
                    // While the indicator runs, per-item info lines would scroll it
                    // off screen; `--verbose` brings them back.
                    let hidden_by_progress =
                        progress.is_active() && !progress::is_verbose() && msg.level == MessageLevel::Info;
                    if hidden_by_progress {
                        continue;
                    }

                    // Append message to buffer, erasing the line it would land on.
                    if msg.should_display() {
                        clear_progress_line(&mut drawn_width);
                        write_message_to_buffer(&mut buffer, &msg);
                    }
                }
                LoggerCommand::Progress(command) => {
                    progress.apply(command);
                    if progress.is_active() {
                        // Messages must land above the line, so flush before drawing it.
                        if last_redraw.is_none_or(|last| last.elapsed() >= REDRAW_INTERVAL) {
                            flush_logs_to_stderr(&mut buffer);
                            redraw_progress_line(&progress, &mut drawn_width, &mut last_redraw, REDRAW_INTERVAL);
                        }
                    } else {
                        clear_progress_line(&mut drawn_width);
                    }
                }
                LoggerCommand::Flush { finished_barrier } => {
                    clear_progress_line(&mut drawn_width);
                    flush_logs_to_stderr(&mut buffer);
                    finished_barrier.wait();
                }
                LoggerCommand::FlushAndShutdown { finished_barrier } => {
                    clear_progress_line(&mut drawn_width);
                    flush_logs_to_stderr(&mut buffer);
                    finished_barrier.wait();
                    return;
                }
            }
        }
    }

    /// Draw the progress line, or erase it when no task is running anymore.
    fn redraw_progress_line(
        progress: &ProgressState,
        drawn_width: &mut usize,
        last_redraw: &mut Option<Instant>,
        redraw_interval: Duration,
    ) {
        if !progress.is_active() {
            clear_progress_line(drawn_width);
            return;
        }

        if !progress.has_work() {
            return;
        }

        if last_redraw.is_some_and(|last| last.elapsed() < redraw_interval) {
            return;
        }
        *last_redraw = Some(Instant::now());

        let line = progress.render(progress::terminal_width());
        let line_width = line.chars().count();
        let padding = drawn_width.saturating_sub(line_width);

        let mut stderr = stderr();
        // Ignore stderr write failures (broken pipe, closed terminal) instead of panicking.
        let _ = write!(stderr, "\r{line}{:padding$}", "", padding = padding);
        *drawn_width = line_width;
    }

    /// Erase the progress line so a log message starts on a clean row.
    fn clear_progress_line(drawn_width: &mut usize) {
        if *drawn_width == 0 {
            return;
        }

        let mut stderr = stderr();
        // Erase with spaces instead of an ANSI escape, so this also works on consoles
        // without escape sequence support.
        let _ = write!(stderr, "\r{:width$}\r", "", width = *drawn_width);
        *drawn_width = 0;
    }

    fn write_message_to_buffer(buffer: &mut Vec<u8>, msg: &PrintMessage) {
        if !contains_unsafe_display_char(&msg.contents) {
            writeln!(buffer, "{msg}").unwrap();
        } else {
            let msg = PrintMessage {
                contents: sanitize_for_display(&msg.contents).into_owned(),
                accessible: msg.accessible,
                level: msg.level,
            };
            writeln!(buffer, "{msg}").unwrap();
        }
    }

    fn flush_logs_to_stderr(buffer: &mut Vec<u8>) {
        if !buffer.is_empty() {
            // Ignore stderr write failures (broken pipe, closed terminal) instead of panicking
            let _ = stderr().write_all(buffer);
            buffer.clear();
        }
    }
}
