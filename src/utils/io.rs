use std::{
    io::{self, stderr, stdout, StderrLock, StdoutLock, Write},
    os::unix::fs::MetadataExt,
};

use fs_err as fs;

use crate::utils::logger;

type StdioOutputLocks = (StdoutLock<'static>, StderrLock<'static>);

pub fn lock_and_flush_output_stdio() -> io::Result<StdioOutputLocks> {
    logger::flush_messages();

    let mut stdout = stdout().lock();
    stdout.flush()?;
    let mut stderr = stderr().lock();
    stderr.flush()?;

    Ok((stdout, stderr))
}

pub fn is_stdin_dev_null() -> io::Result<bool> {
    let stdin = fs::metadata("/proc/self/fd/0")?;
    let null = fs::metadata("/dev/null")?;
    Ok(stdin.dev() == null.dev() && stdin.ino() == null.ino())
}

/// Workaround for `dyn Read + Seek`
pub trait ReadSeek: io::Read + io::Seek {}
impl<T> ReadSeek for T where T: io::Read + io::Seek {}
