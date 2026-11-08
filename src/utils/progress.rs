//! Progress indicator drawn on `stderr` while compressing or decompressing.
//!
//! # Why a hand-rolled indicator?
//!
//! `ouch` used to depend on `indicatif`, but the progress bar was removed in
//! <https://github.com/ouch-org/ouch/pull/300> and the dependency with it. What
//! is needed today is a single status line, with no multi-bar layout, no
//! templates and no styling: hand-rolling it keeps the dependency tree small
//! and lets the line be owned by the logging thread, which is the only writer
//! of `stderr` since <https://github.com/ouch-org/ouch/issues/643>.
//!
//! # Rules
//!
//! - The line is only drawn when `stderr` is a terminal, `--quiet` and
//!   `--verbose` are absent, and accessibility mode is off. Every other case
//!   falls back to the usual logging output, so piped output is unchanged.
//! - Worker threads never touch the terminal: they send [`ProgressCommand`]s
//!   through the logging channel and the logging thread renders them. This
//!   keeps the indicator from tearing log lines apart.
//! - The line contains no ANSI escapes and no control characters. It is
//!   redrawn with `\r` and padded with spaces over the previous line, which
//!   also works on Windows consoles where escape sequences are unavailable.
//! - Counters are aggregated over every task running at the same time: `ouch`
//!   unpacks multiple inputs in parallel, and one combined line is calmer than
//!   several bars fighting for the same terminal row.

use std::{
    io::{self, IsTerminal, Read, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use super::{BytesFmt, formatting::sanitize_for_display, logger};
use crate::accessible::is_running_in_accessible_mode;

/// Width of the bar, in characters, on terminals wide enough to have one.
const BAR_WIDTH: usize = 20;
/// Terminals narrower than this only get a spinner and counters, no bar.
const MIN_WIDTH_FOR_BAR: usize = 60;
/// Assumed width when the real one cannot be queried (Windows, redirected).
const DEFAULT_TERMINAL_WIDTH: usize = 100;
/// Below this many columns there is no room left for the current item.
const MIN_MESSAGE_CHARS: usize = 8;

/// Spinner frames used when the total amount of work is unknown.
const SPINNER: [char; 4] = ['-', '\\', '|', '/'];

/// Whether the indicator is allowed to draw (set from the CLI flags).
static ENABLED: AtomicBool = AtomicBool::new(false);
/// Whether `--verbose` was passed, which restores per-item log lines.
static VERBOSE: AtomicBool = AtomicBool::new(false);

/// Read the CLI flags and decide whether a progress line may be drawn.
///
/// Must be called after the accessibility flag is set, and before any
/// compression or decompression starts.
pub fn configure(quiet: bool, verbose: bool) {
    let supported = !quiet && !verbose && io::stderr().is_terminal() && !is_running_in_accessible_mode();
    ENABLED.store(supported, Ordering::Relaxed);
    VERBOSE.store(verbose, Ordering::Relaxed);
}

/// Whether `--verbose` was passed.
pub(crate) fn is_verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}

fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// What the counter on the progress line counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// Items, such as files inside an archive.
    Items,
    /// Bytes, used when streaming a single file.
    Bytes,
}

impl Unit {
    fn format_value(self, value: u64) -> String {
        match self {
            Self::Items => value.to_string(),
            Self::Bytes => BytesFmt(value).to_string(),
        }
    }

    fn noun(self, value: u64) -> &'static str {
        match self {
            Self::Items if value == 1 => " file",
            Self::Items => " files",
            Self::Bytes => "",
        }
    }
}

/// A task reporting into the shared progress line.
///
/// Dropping the last handle of a task ends it and redraws the remaining ones.
/// The handle is inert when progress output is disabled, so call sites do not
/// have to branch.
#[derive(Clone)]
pub struct Progress {
    // `None` means "disabled": every method is then a no-op.
    guard: Option<Arc<ProgressGuard>>,
}

/// Sends the `Finish` command exactly once, when the last [`Progress`] handle
/// of a task is dropped, including on early returns and error paths.
struct ProgressGuard;

impl Drop for ProgressGuard {
    fn drop(&mut self) {
        // The logger thread may already be gone during shutdown; never panic from a drop.
        logger::send_progress_command(ProgressCommand::Finish);
    }
}

impl Progress {
    /// Start a task whose total amount of work is not known upfront.
    pub fn start(label: &'static str, unit: Unit) -> Self {
        if !is_enabled() {
            return Self { guard: None };
        }

        logger::send_progress_command(ProgressCommand::Start { label, unit });

        Self {
            guard: Some(Arc::new(ProgressGuard)),
        }
    }

    /// Start a task that knows how much work it will do, which draws a filled bar.
    pub fn start_with_total(label: &'static str, unit: Unit, total: u64) -> Self {
        let progress = Self::start(label, unit);
        progress.set_total(total);
        progress
    }

    /// Declare how much work this task has left, enabling the filled bar.
    pub fn set_total(&self, total: u64) {
        if self.guard.is_some() {
            logger::send_progress_command(ProgressCommand::SetTotal(total));
        }
    }

    /// Report `amount` more units of work done.
    pub fn advance(&self, amount: u64) {
        if self.guard.is_some() {
            logger::send_progress_command(ProgressCommand::Advance(amount));
        }
    }

    /// Set the item currently being worked on, shown at the end of the line.
    pub fn set_message(&self, message: impl Into<String>) {
        if self.guard.is_some() {
            logger::send_progress_command(ProgressCommand::SetMessage(message.into()));
        }
    }
}

/// A [`Read`] adapter reporting the bytes it hands out.
pub struct ProgressReader<R> {
    inner: R,
    progress: Progress,
}

impl<R> ProgressReader<R> {
    pub fn new(inner: R, progress: &Progress) -> Self {
        Self {
            inner,
            progress: progress.clone(),
        }
    }
}

impl<R: Read> Read for ProgressReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buf)?;
        // `usize` never exceeds `u64` on the platforms `ouch` supports.
        self.progress.advance(read as u64);
        Ok(read)
    }
}

/// A [`Write`] adapter reporting the bytes written through it.
pub struct ProgressWriter<W> {
    inner: W,
    progress: Progress,
}

impl<W> ProgressWriter<W> {
    pub fn new(inner: W, progress: &Progress) -> Self {
        Self {
            inner,
            progress: progress.clone(),
        }
    }
}

impl<W: Write> Write for ProgressWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buf)?;
        // `usize` never exceeds `u64` on the platforms `ouch` supports.
        self.progress.advance(written as u64);
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Messages sent from worker threads to the logging thread.
#[derive(Debug)]
pub(crate) enum ProgressCommand {
    /// A new task started.
    Start { label: &'static str, unit: Unit },
    /// The task declared how much work it has in total.
    SetTotal(u64),
    /// Work was done.
    Advance(u64),
    /// The item currently being worked on changed.
    SetMessage(String),
    /// A task ended.
    Finish,
}

/// The state rendered by the logging thread.
///
/// Counters are shared by every task running at the same time, see the module
/// documentation.
#[derive(Debug)]
pub(crate) struct ProgressState {
    active: usize,
    tasks_with_total: usize,
    label: &'static str,
    unit: Unit,
    done: u64,
    total: u64,
    message: String,
    spinner: usize,
}

impl Default for ProgressState {
    fn default() -> Self {
        Self {
            active: 0,
            tasks_with_total: 0,
            label: "",
            unit: Unit::Items,
            done: 0,
            total: 0,
            message: String::new(),
            spinner: 0,
        }
    }
}

impl ProgressState {
    /// Apply a command sent by a worker thread.
    pub(crate) fn apply(&mut self, command: ProgressCommand) {
        match command {
            ProgressCommand::Start { label, unit } => {
                // The first task to start owns the label, later ones only add counters.
                if self.active == 0 {
                    *self = Self {
                        label,
                        unit,
                        ..Self::default()
                    };
                }
                self.active += 1;
            }
            ProgressCommand::SetTotal(total) => {
                self.total = self.total.saturating_add(total);
                self.tasks_with_total += 1;
            }
            ProgressCommand::Advance(amount) => {
                self.done = self.done.saturating_add(amount);
                // Animate only when work happens, so an idle task keeps its frame.
                self.spinner = self.spinner.wrapping_add(1);
            }
            ProgressCommand::SetMessage(message) => self.message = message,
            ProgressCommand::Finish => {
                self.active = self.active.saturating_sub(1);
                if self.active == 0 {
                    let (label, unit) = (self.label, self.unit);
                    *self = Self {
                        label,
                        unit,
                        ..Self::default()
                    };
                }
            }
        }
    }

    /// Whether at least one task is running.
    pub(crate) fn is_active(&self) -> bool {
        self.active > 0
    }

    /// Whether there is any work to report yet.
    ///
    /// The line stays hidden until the first unit is done: an operation short
    /// enough to finish in a few milliseconds would otherwise flash a zeroed
    /// line and erase it, which is worse than the logs it replaced.
    pub(crate) fn has_work(&self) -> bool {
        self.done > 0
    }

    /// Render the line shown on the terminal, at most `width` columns wide.
    ///
    /// The result is plain text: no ANSI escapes, no control characters, and
    /// sanitized file names.
    pub(crate) fn render(&self, width: usize) -> String {
        if !self.is_active() {
            return String::new();
        }

        let determinate = self.total > 0 && self.tasks_with_total == self.active;
        let bar_width = if determinate && width >= MIN_WIDTH_FOR_BAR {
            BAR_WIDTH
        } else {
            0
        };

        let mut line = String::with_capacity(96);
        line.push_str(self.label);
        line.push(' ');

        if bar_width > 0 {
            let done = self.done.min(self.total);
            #[expect(
                clippy::cast_precision_loss,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "bar_width is a small constant, the ratio rounds into 0..=bar_width"
            )]
            let filled = ((done as f64 / self.total as f64) * bar_width as f64).round() as usize;

            line.push('[');
            for index in 0..bar_width {
                line.push(if index < filled { '#' } else { '-' });
            }
            line.push(']');
            line.push(' ');
        } else if !determinate {
            line.push('[');
            line.push(SPINNER[self.spinner % SPINNER.len()]);
            line.push_str("] ");
        }

        let shown = if determinate {
            self.done.min(self.total)
        } else {
            self.done
        };
        if determinate {
            line.push_str(&self.unit.format_value(shown));
            line.push('/');
            line.push_str(&self.unit.format_value(self.total));
        } else {
            line.push_str(&self.unit.format_value(shown));
        }
        // The noun agrees with the total when there is one.
        line.push_str(self.unit.noun(if determinate { self.total } else { shown }));

        let fixed_length = line.chars().count();
        let budget = width.saturating_sub(fixed_length + 2);
        if budget >= MIN_MESSAGE_CHARS {
            let message = truncate_message(&self.message, budget);
            if !message.is_empty() {
                line.push_str("  ");
                line.push_str(&message);
            }
        }

        line
    }
}

/// Keep the last `budget` characters of `message`, which for paths is the
/// informative part, and drop characters that could corrupt the terminal.
fn truncate_message(message: &str, budget: usize) -> String {
    let sanitized = sanitize_for_display(message);
    let char_count = sanitized.chars().count();
    if char_count <= budget {
        return sanitized.into_owned();
    }

    // Three columns are spent on the ellipsis, and `budget` is at least
    // `MIN_MESSAGE_CHARS`.
    let keep = budget.saturating_sub(3);
    let tail: String = sanitized.chars().skip(char_count - keep).collect();
    format!("...{tail}")
}

/// Best-effort width of the terminal `stderr` writes to.
///
/// Falls back to [`DEFAULT_TERMINAL_WIDTH`] when the size cannot be queried.
pub(crate) fn terminal_width() -> usize {
    #[cfg(unix)]
    {
        // SAFETY: `TIOCGWINSZ` writes a `winsize` into the pointer passed to
        // `ioctl`; `size` is a live, properly aligned and initialized
        // `winsize` for the duration of the call. On failure `ioctl` returns
        // a nonzero value and leaves the layout readable, which the check
        // below handles.
        let result = unsafe {
            let mut size = libc::winsize {
                ws_row: 0,
                ws_col: 0,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            libc::ioctl(libc::STDERR_FILENO, libc::TIOCGWINSZ, &mut size);
            size.ws_col
        };

        if result > 0 {
            return usize::from(result);
        }
    }

    DEFAULT_TERMINAL_WIDTH
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_with(state: &ProgressState, width: usize) -> String {
        state.render(width)
    }

    fn state_of(commands: &[ProgressCommand]) -> ProgressState {
        let mut state = ProgressState::default();
        for command in commands {
            state.apply(command_for_test(command));
        }
        state
    }

    // Commands are not `Clone`, so tests build their own copies.
    fn command_for_test(command: &ProgressCommand) -> ProgressCommand {
        match command {
            ProgressCommand::Start { label, unit } => ProgressCommand::Start { label, unit: *unit },
            ProgressCommand::SetTotal(total) => ProgressCommand::SetTotal(*total),
            ProgressCommand::Advance(amount) => ProgressCommand::Advance(*amount),
            ProgressCommand::SetMessage(message) => ProgressCommand::SetMessage(message.clone()),
            ProgressCommand::Finish => ProgressCommand::Finish,
        }
    }

    #[test]
    fn renders_a_filled_bar_when_the_total_is_known() {
        let state = state_of(&[
            ProgressCommand::Start {
                label: "Decompressing",
                unit: Unit::Items,
            },
            ProgressCommand::SetTotal(100),
            ProgressCommand::Advance(50),
        ]);

        let line = render_with(&state, 100);

        assert_eq!(line, "Decompressing [##########----------] 50/100 files");
    }

    #[test]
    fn renders_a_spinner_when_the_total_is_unknown() {
        let state = state_of(&[
            ProgressCommand::Start {
                label: "Compressing",
                unit: Unit::Items,
            },
            ProgressCommand::Advance(7),
        ]);

        let line = render_with(&state, 100);

        // The spinner frame is decoration; the counts are the contract.
        assert!(line.starts_with("Compressing ["));
        assert!(line.ends_with("] 7 files"), "unexpected line: {line}");
    }

    #[test]
    fn renders_bytes_with_human_readable_units() {
        let state = state_of(&[
            ProgressCommand::Start {
                label: "Compressing",
                unit: Unit::Bytes,
            },
            ProgressCommand::SetTotal(2_000_000),
            ProgressCommand::Advance(1_000_000),
        ]);

        let line = render_with(&state, 100);

        assert_eq!(line, "Compressing [##########----------]   1.00 MiB/  2.00 MiB");
    }

    #[test]
    fn falls_back_to_a_spinner_when_one_task_lacks_a_total() {
        let state = state_of(&[
            ProgressCommand::Start {
                label: "Decompressing",
                unit: Unit::Items,
            },
            ProgressCommand::SetTotal(100),
            ProgressCommand::Advance(10),
            // A second, indeterminate task: the combined line cannot claim a bar.
            ProgressCommand::Start {
                label: "Decompressing",
                unit: Unit::Items,
            },
            ProgressCommand::Advance(5),
        ]);

        let line = render_with(&state, 100);

        assert!(!line.contains("---"), "bar must not be drawn: {line}");
        assert!(line.contains("15 files"), "counters must be aggregated: {line}");
    }

    #[test]
    fn keeps_the_message_when_it_fits() {
        let state = state_of(&[
            ProgressCommand::Start {
                label: "Compressing",
                unit: Unit::Items,
            },
            ProgressCommand::Advance(1),
            ProgressCommand::SetMessage("src/utils/progress.rs".to_owned()),
        ]);

        let line = render_with(&state, 100);

        assert!(line.ends_with("  src/utils/progress.rs"), "unexpected line: {line}");
    }

    #[test]
    fn keeps_the_end_of_a_message_that_does_not_fit() {
        let state = state_of(&[
            ProgressCommand::Start {
                label: "Compressing",
                unit: Unit::Items,
            },
            ProgressCommand::Advance(1),
            ProgressCommand::SetMessage("a/very/long/path/that/keeps/going/and/going/file.txt".to_owned()),
        ]);

        let line = render_with(&state, 60);

        assert!(line.chars().count() <= 60, "line too wide: {line}");
        assert!(line.contains("..."), "long messages must be elided: {line}");
        assert!(line.ends_with("file.txt"), "the informative tail must stay: {line}");
    }

    #[test]
    fn drops_the_bar_on_narrow_terminals() {
        let state = state_of(&[
            ProgressCommand::Start {
                label: "Decompressing",
                unit: Unit::Items,
            },
            ProgressCommand::SetTotal(100),
            ProgressCommand::Advance(50),
        ]);

        let line = render_with(&state, 40);

        assert!(!line.contains('#'), "no bar fits in 40 columns: {line}");
        assert!(line.contains("50/100 files"), "counters must stay: {line}");
    }

    #[test]
    fn drops_unsafe_display_characters() {
        let state = state_of(&[
            ProgressCommand::Start {
                label: "Compressing",
                unit: Unit::Items,
            },
            ProgressCommand::SetMessage("evil\u{1b}[2Jname".to_owned()),
        ]);

        let line = render_with(&state, 100);

        assert!(
            !line.contains('\u{1b}'),
            "escape sequences must not reach the terminal: {line}"
        );
    }

    #[test]
    fn resets_counters_after_the_last_task_finishes() {
        let mut state = state_of(&[
            ProgressCommand::Start {
                label: "Compressing",
                unit: Unit::Items,
            },
            ProgressCommand::Advance(3),
            ProgressCommand::Finish,
        ]);

        assert!(!state.is_active());
        assert!(render_with(&state, 100).is_empty());

        // A later task starts from a clean line instead of inheriting counters.
        state.apply(ProgressCommand::Start {
            label: "Decompressing",
            unit: Unit::Items,
        });
        assert_eq!(render_with(&state, 100), "Decompressing [-] 0 files");
    }

    #[test]
    fn a_disabled_handle_is_inert() {
        // `Progress::start` returns this shape when progress output is off:
        // every call must be a no-op instead of a terminal write.
        let progress = Progress { guard: None };

        progress.set_total(1);
        progress.advance(1);
        progress.set_message("ignored");
        drop(progress);
    }
}
