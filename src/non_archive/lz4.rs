use std::io::{self, Read};

use lz4_flex::frame::FrameDecoder;

struct PrependReader<R> {
    prefix: Option<u8>,
    inner: R,
}

impl<R: Read> Read for PrependReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(slot) = buf.first_mut() {
            if let Some(byte) = self.prefix.take() {
                *slot = byte;
                return Ok(1);
            }
        }
        self.inner.read(buf)
    }
}

/// Wrapper that handles concatenated lz4 frames (the standard FrameDecoder only reads one).
pub struct MultiFrameLz4Decoder<R: Read> {
    decoder: Option<FrameDecoder<PrependReader<R>>>,
}

impl<R: Read> MultiFrameLz4Decoder<R> {
    pub fn new(reader: R) -> Self {
        Self {
            decoder: Some(FrameDecoder::new(PrependReader { prefix: None, inner: reader })),
        }
    }
}

impl<R: Read> Read for MultiFrameLz4Decoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            let Some(decoder) = &mut self.decoder else {
                return Ok(0);
            };

            let n = decoder.read(buf)?;
            if n > 0 {
                return Ok(n);
            }

            // Frame finished, check for next frame.
            let mut inner = self.decoder.take().unwrap().into_inner().inner;
            let mut peek = [0u8];
            if inner.read(&mut peek)? == 0 {
                return Ok(0);
            }

            self.decoder = Some(FrameDecoder::new(PrependReader {
                prefix: Some(peek[0]),
                inner,
            }));
        }
    }
}
