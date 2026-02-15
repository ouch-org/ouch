use std::io::{self, stderr, stdout, StderrLock, StdoutLock, Write};

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

pub trait ReadSeek: io::Read + io::Seek {}
impl<T> ReadSeek for T where T: io::Read + io::Seek {}
