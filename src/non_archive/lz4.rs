use std::io::{self, Read};

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

        for frame_index in 0..u32::MAX {
            let pos = cursor.position() as usize;
            let remaining = cursor.get_ref().len() - pos;

            if remaining == 0 {
                break;
            }

            if remaining < 4 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    format!("Incomplete LZ4 frame header (frame index: {})", frame_index),
                ));
            }

            // Check for magic number
            let slice = &cursor.get_ref()[pos..pos + 4];
            if slice != LZ4_MAGIC {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid LZ4 frame magic number (frame index: {})", frame_index),
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
