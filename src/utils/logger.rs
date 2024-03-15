use std::sync::{
    mpsc::{channel, Receiver, Sender},
    OnceLock,
};

use super::colors::{ORANGE, RESET, YELLOW};
use crate::accessible::is_running_in_accessible_mode;

static SENDER: OnceLock<Sender<PrintMessage>> = OnceLock::new();

pub fn setup_channel() -> Receiver<PrintMessage> {
    let (tx, rx) = channel();
    SENDER.set(tx).expect("`setup_channel` should only be called once");
    rx
}

#[track_caller]
fn get_sender() -> &'static Sender<PrintMessage> {
    SENDER.get().expect("No sender, you need to call `setup_channel` first")
}

/// Message object used for sending logs from worker threads to a logging thread via channels.
/// See <https://github.com/ouch-org/ouch/issues/643>
#[derive(Debug)]
pub struct PrintMessage {
    contents: String,
    accessible: bool,
    level: MessageLevel,
}

pub fn map_message(msg: &PrintMessage) -> Option<String> {
    match msg.level {
        MessageLevel::Info => {
            if msg.accessible {
                if is_running_in_accessible_mode() {
                    Some(format!("{}Info:{} {}", *YELLOW, *RESET, msg.contents))
                } else {
                    Some(format!("{}[INFO]{} {}", *YELLOW, *RESET, msg.contents))
                }
            } else if !is_running_in_accessible_mode() {
                Some(format!("{}[INFO]{} {}", *YELLOW, *RESET, msg.contents))
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

/// An `[INFO]` log to be displayed if we're not running accessibility mode.
///
/// Same as `.info_accessible()`, but only displayed if accessibility mode
/// is turned off, which is detected by the function
/// `is_running_in_accessible_mode`.
///
/// Read more about accessibility mode in `accessible.rs`.
pub fn info(contents: String) {
    info_with_accessibility(contents, false);
}

/// An `[INFO]` log to be displayed.
///
/// Same as `.info()`, but also displays if `is_running_in_accessible_mode`
/// returns `true`.
///
/// Read more about accessibility mode in `accessible.rs`.
pub fn info_accessible(contents: String) {
    info_with_accessibility(contents, true);
}

fn info_with_accessibility(contents: String, accessible: bool) {
    get_sender()
        .send(PrintMessage {
            contents,
            accessible,
            level: MessageLevel::Info,
        })
        .unwrap();
}

pub fn warning(contents: String) {
    get_sender()
        .send(PrintMessage {
            contents,
            // Warnings are important and unlikely to flood, so they should be displayed
            accessible: true,
            level: MessageLevel::Warning,
        })
        .unwrap();
}

#[derive(Clone)]
pub struct Logger {
    log_sender: Sender<PrintMessage>,
}

impl Logger {
    pub fn new(log_sender: Sender<PrintMessage>) -> Self {
        Self { log_sender }
    }

    /// An `[INFO]` log to be displayed if we're not running accessibility mode.
    ///
    /// Same as `.info_accessible()`, but only displayed if accessibility mode
    /// is turned off, which is detected by the function
    /// `is_running_in_accessible_mode`.
    ///
    /// Read more about accessibility mode in `accessible.rs`.
    pub fn info(&self, contents: String) {
        self.info_with_accessibility(contents, false);
    }

    /// An `[INFO]` log to be displayed.
    ///
    /// Same as `.info()`, but also displays if `is_running_in_accessible_mode`
    /// returns `true`.
    ///
    /// Read more about accessibility mode in `accessible.rs`.
    pub fn info_accessible(&self, contents: String) {
        self.info_with_accessibility(contents, true);
    }

    fn info_with_accessibility(&self, contents: String, accessible: bool) {
        self.log_sender
            .send(PrintMessage {
                contents,
                accessible,
                level: MessageLevel::Info,
            })
            .unwrap();
    }

    pub fn warning(&self, contents: String) {
        self.log_sender
            .send(PrintMessage {
                contents,
                // Warnings are important and unlikely to flood, so they should be displayed
                accessible: true,
                level: MessageLevel::Warning,
            })
            .unwrap();
    }
}

#[derive(Debug, PartialEq)]
pub enum MessageLevel {
    Info,
    Warning,
}
