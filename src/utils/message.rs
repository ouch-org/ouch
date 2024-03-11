/// Message object used for sending logs from worker threads to a logging thread via channels.
/// See <https://github.com/ouch-org/ouch/issues/632>
///
/// ## Example
///
/// ```rs
/// // This is already done in the main thread in src/commands/mod.rs
/// // Functions that want to log anything just need to have
/// // `log_sender: Sender<PrintMessage>` as an argument.
/// let (log_sender, log_receiver) = channel::<PrintMessage>();
///
/// log_sender
///   .send(PrintMessage {
///       contents: "Hello, world!".to_string(),
///       accessible: true,
///   }).unwrap();
/// ```
#[derive(Debug)]
pub struct PrintMessage {
    pub contents: String,
    pub accessible: bool,
    pub level: MessageLevel,
}

#[derive(Debug, PartialEq)]
pub enum MessageLevel {
    Info,
    Warning,
}
