use std::{
    io::{self, StderrLock, StdoutLock, Write, stderr, stdout},
    sync::atomic::{AtomicBool, Ordering},
};

#[cfg(unix)]
use fs_err as fs;

use crate::utils::logger;

type StdioOutputLocks = (StdoutLock<'static>, StderrLock<'static>);

/// Set when stdin carries the archive and prompts must not read answers from it.
static STDIN_IS_INPUT: AtomicBool = AtomicBool::new(false);

pub fn set_stdin_is_input() {
    STDIN_IS_INPUT.store(true, Ordering::Relaxed);
}

pub fn is_stdin_input() -> bool {
    STDIN_IS_INPUT.load(Ordering::Relaxed)
}

pub fn lock_and_flush_output_stdio() -> io::Result<StdioOutputLocks> {
    logger::flush_messages();

    let mut stdout = stdout().lock();
    stdout.flush()?;
    let mut stderr = stderr().lock();
    stderr.flush()?;

    Ok((stdout, stderr))
}

#[cfg(unix)]
pub fn is_stdin_dev_null() -> io::Result<bool> {
    use std::os::unix::fs::MetadataExt;

    let stdin = fs::metadata("/dev/stdin")?;
    let null = fs::metadata("/dev/null")?;
    Ok(stdin.dev() == null.dev() && stdin.ino() == null.ino())
}

#[cfg(not(unix))]
pub fn is_stdin_dev_null() -> io::Result<bool> {
    Ok(false)
}

/// Workaround for `dyn Read + Seek`
pub trait ReadSeek: io::Read + io::Seek {}
impl<T> ReadSeek for T where T: io::Read + io::Seek {}
