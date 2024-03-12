use std::sync::mpsc::Sender;

use super::colors::{ORANGE, RESET, YELLOW};
use crate::accessible::is_running_in_accessible_mode;

/// Message object used for sending logs from worker threads to a logging thread via channels.
/// See <https://github.com/ouch-org/ouch/issues/632>
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

#[derive(Clone)]
pub struct Logger {
    log_sender: Sender<PrintMessage>,
}

impl Logger {
    pub fn new(log_sender: Sender<PrintMessage>) -> Self {
        Self { log_sender }
    }

    pub fn info(&self, contents: String, accessible: bool) {
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
                accessible: true, // does not matter
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
