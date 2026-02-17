use std::io::{self, Read};

use lz4_flex::frame::FrameDecoder;

pub struct MultiFrameLz4Decoder<R: Read> {
    decoder: FrameDecoder<R>,
}

impl<R: Read> MultiFrameLz4Decoder<R> {
    pub fn new(reader: R) -> Self {
        Self {
            decoder: FrameDecoder::new(reader),
        }
    }
}

impl<R: Read> Read for MultiFrameLz4Decoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.decoder.read(buf)? {
            // got EOF once, try again
            0 => self.decoder.read(buf),
            bytes => Ok(bytes),
        }
    }
}
