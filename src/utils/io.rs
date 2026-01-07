use std::io::{self, stderr, stdout, Read, StderrLock, StdoutLock, Write};

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

/// A wrapper around lz4_flex::frame::FrameDecoder that handles concatenated lz4 frames.
/// The standard FrameDecoder only reads a single frame and returns EOF.
/// This wrapper continues reading subsequent frames until the underlying reader is exhausted.
pub struct MultiFrameLz4Decoder {
    buffer: io::Cursor<Vec<u8>>,
}

impl MultiFrameLz4Decoder {
    pub fn new(mut reader: impl Read) -> io::Result<Self> {
        // Decompress all concatenated frames into an in-memory buffer
        let mut output = Vec::new();
        let mut input_buffer = Vec::new();
        reader.read_to_end(&mut input_buffer)?;

        let mut cursor = io::Cursor::new(input_buffer);

        // LZ4 frame magic number (little-endian)
        const LZ4_MAGIC: [u8; 4] = [0x04, 0x22, 0x4D, 0x18];

        loop {
            let pos = cursor.position() as usize;
            let remaining = cursor.get_ref().len() - pos;

            if remaining == 0 {
                break;
            }

            if remaining < 4 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "Incomplete LZ4 frame header",
                ));
            }

            // Check for magic number
            let slice = &cursor.get_ref()[pos..pos + 4];
            if slice != LZ4_MAGIC {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid LZ4 frame magic number",
                ));
            }

            // Create a decoder for this frame starting from current position
            cursor.set_position(pos as u64);
            let frame_reader = io::Cursor::new(&cursor.get_ref()[pos..]);
            let mut decoder = lz4_flex::frame::FrameDecoder::new(frame_reader);

            // Read the frame
            let start_len = output.len();
            decoder.read_to_end(&mut output)?;

            // Get how many bytes were consumed from the input
            let bytes_consumed = decoder.into_inner().position() as usize;
            cursor.set_position((pos + bytes_consumed) as u64);

            // If no progress was made, break to avoid infinite loop
            if output.len() == start_len && bytes_consumed == 0 {
                break;
            }
        }

        Ok(Self {
            buffer: io::Cursor::new(output),
        })
    }
}

impl Read for MultiFrameLz4Decoder {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.buffer.read(buf)
    }
}
