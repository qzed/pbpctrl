use bytes::{Buf, BytesMut};

use super::consts;
use super::crc;
use super::varint;
use super::Frame;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    UnexpectedData,
    UnexpectedEndOfFrame,
    InvalidChecksum,
    InvalidEncoding,
    InvalidFrame,
    InvalidAddress,
    BufferOverflow,
}

impl From<varint::DecodeError> for Error {
    fn from(value: varint::DecodeError) -> Self {
        match value {
            varint::DecodeError::Incomplete => Self::InvalidFrame,
            varint::DecodeError::Overflow => Self::InvalidAddress,
        }
    }
}


#[derive(Debug)]
pub struct Decoder {
    buf: Vec<u8>,
    state: (State, EscState),
    current_frame_size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Discard,
    Frame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EscState {
    Normal,
    Escape,
}

impl Decoder {
    pub fn new() -> Self {
        Self::with_capacity(4096)
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            buf: Vec::with_capacity(cap),
            state: (State::Discard, EscState::Normal),
            current_frame_size: 0,
        }
    }

    pub fn process(&mut self, buf: &mut BytesMut) -> Result<Option<Frame>, Error> {
        if buf.is_empty() {
            return Ok(None);
        }

        loop {
            match self.state.0 {
                State::Discard => {
                    // try to find the start of this frame
                    match find_frame_start(buf) {
                        // expected: immediate start of frame
                        Some(0) => {
                            self.state.0 = State::Frame;
                            buf.advance(1);
                        },
                        // unexpected: n bytes before start of frame
                        Some(n) => {
                            self.state.0 = State::Frame;
                            buf.advance(n + 1);
                            return Err(Error::UnexpectedData);
                        },
                        // unexpected: unknown amount of bytes before start of frame
                        None => {
                            // check whether the last byte might indicate a start
                            let n = if buf.last() == Some(&consts::flags::FRAME) {
                                buf.len() - 1
                            } else {
                                buf.len()
                            };

                            buf.advance(n);
                            return Err(Error::UnexpectedData);
                        },
                    }
                },
                State::Frame => {
                    // copy and decode to internal buffer
                    for (i, b) in buf.iter().copied().enumerate() {
                        match (b, self.state.1) {
                            (consts::flags::ESCAPE, EscState::Normal) => {
                                self.state.1 = EscState::Escape;
                            },
                            (consts::flags::ESCAPE, EscState::Escape) => {
                                buf.advance(i + 1);
                                self.reset();

                                return Err(Error::InvalidEncoding);
                            },
                            (consts::flags::FRAME, EscState::Normal) => {
                                buf.advance(i + 1);

                                return self.decode_buffered();
                            },
                            (consts::flags::FRAME, EscState::Escape) => {
                                buf.advance(i);
                                self.reset();

                                return Err(Error::UnexpectedEndOfFrame);
                            },
                            (b, EscState::Normal) => {
                                self.push_byte(b);
                            },
                            (b, EscState::Escape) => {
                                self.push_byte(b ^ consts::escape::MASK);
                                self.state.1 = EscState::Normal;
                            },
                        }
                    }

                    buf.advance(buf.remaining());
                    return Ok(None);
                },
            }
        }
    }

    fn decode_buffered(&mut self) -> Result<Option<Frame>, Error> {
        // validate minimum frame size
        if self.buf.len() < 6 {
            self.reset();
            self.state.0 = State::Frame;        // the next frame may already start
            return Err(Error::InvalidFrame);
        }

        // validate checksum
        let crc_actual = crc::crc32(&self.buf[..self.buf.len()-4]);
        let crc_expect = self.buf[self.buf.len()-4..].try_into().unwrap();
        let crc_expect = u32::from_le_bytes(crc_expect);

        if crc_expect != crc_actual {
            self.reset();
            self.state.0 = State::Frame;        // the next frame may already start
            return Err(Error::InvalidChecksum);
        }

        // check for overflow
        if self.current_frame_size > self.buf.len() {
            self.reset();
            return Err(Error::BufferOverflow);
        }

        // decode address
        let (address, n) = varint::decode(&self.buf)?;

        // validate minimum remaining frame size
        if self.buf.len() < n + 5 {
            self.reset();
            return Err(Error::InvalidFrame);
        }

        // get control byte and data
        let control = self.buf[n];
        let data = self.buf[n+1..self.buf.len()-4].into();

        let frame = Frame {
            address,
            control,
            data,
        };

        self.reset();
        Ok(Some(frame))
    }

    fn push_byte(&mut self, byte: u8) {
        self.current_frame_size += 1;

        if self.buf.len() < self.buf.capacity() {
            self.buf.push(byte);
        }
    }

    fn reset(&mut self) {
        self.buf.clear();
        self.state = (State::Discard, EscState::Normal);
        self.current_frame_size = 0;
    }
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}


fn find_frame_start(buf: &[u8]) -> Option<usize> {
    buf.windows(2)
        .enumerate()
        .find(|(_, b)| b[0] == consts::flags::FRAME && b[1] != consts::flags::FRAME)
        .map(|(i, _)| i)
}


#[cfg(test)]
mod test {
    use bytes::BufMut;

    use super::*;

    #[test]
    fn test_find_frame_start() {
        let buf = [0x7E, 0x01, 0x02, 0x03];
        assert_eq!(find_frame_start(&buf), Some(0));

        let buf = [0x03, 0x02, 0x01, 0x00, 0x7E, 0x00, 0x01, 0x02, 0x03];
        assert_eq!(find_frame_start(&buf), Some(4));

        let buf = [0x03, 0x02, 0x01, 0x00, 0x7E, 0x7E, 0x00, 0x01, 0x02, 0x03];
        assert_eq!(find_frame_start(&buf), Some(5));

        let buf = [0x03, 0x02, 0x01, 0x00, 0x7E];
        assert_eq!(find_frame_start(&buf), None);

        let buf = [0x03, 0x02, 0x01, 0x00, 0x7E, 0x00];
        assert_eq!(find_frame_start(&buf), Some(4));

        let buf = [0x7E];
        assert_eq!(find_frame_start(&buf), None);

        let buf = [];
        assert_eq!(find_frame_start(&buf), None);
    }

    #[test]
    fn test_frame_decode() {
        let data = [
            // message
            0x7e, 0x06, 0x08, 0x09, 0x03, 0x05, 0x06, 0x07, 0x7d, 0x5d,
            0x7d, 0x5e, 0x7f, 0xff, 0xe6, 0x2d, 0x17, 0xc6, 0x7e,
            // and trailing bytes
            0x02, 0x01
        ];

        let expect = Frame {
            address: 0x010203,
            control: 0x03,
            data: vec![0x05, 0x06, 0x07, 0x7D, 0x7E, 0x7F, 0xFF].into(),
        };

        let mut dec = Decoder::new();

        // test standard decoding
        let mut buf = BytesMut::from(&data[..data.len()-2]);
        assert_eq!(dec.process(&mut buf), Ok(Some(expect.clone())));
        assert_eq!(buf.remaining(), 0);

        // test decoding with trailing bytes
        let mut buf = BytesMut::from(&data[..data.len()]);
        assert_eq!(dec.process(&mut buf), Ok(Some(expect.clone())));
        assert_eq!(buf.remaining(), 2);

        assert_eq!(dec.process(&mut buf), Err(Error::UnexpectedData));
        assert_eq!(buf.remaining(), 0);

        // test partial decoding / re-entrancy
        let mut buf = BytesMut::from(&data[..9]);
        assert_eq!(dec.process(&mut buf), Ok(None));
        assert_eq!(buf.remaining(), 0);

        assert_eq!(dec.state, (State::Frame, EscState::Escape));

        let mut buf = BytesMut::from(&data[9..data.len()-2]);
        assert_eq!(dec.process(&mut buf), Ok(Some(expect.clone())));
        assert_eq!(buf.remaining(), 0);

        // test decoding of subsequent frames
        let mut buf = BytesMut::new();
        buf.put_slice(&data[..data.len()-2]);
        buf.put_slice(&data[..]);

        assert_eq!(dec.process(&mut buf), Ok(Some(expect.clone())));
        assert_eq!(buf.remaining(), data.len());

        assert_eq!(dec.process(&mut buf), Ok(Some(expect.clone())));
        assert_eq!(buf.remaining(), 2);

        // test decoding of cut-off frame / data loss (with frame being too small)
        let mut buf = BytesMut::new();
        buf.put_slice(&data[..5]);
        buf.put_slice(&data[..]);

        assert_eq!(dec.process(&mut buf), Err(Error::InvalidFrame));
        assert_eq!(dec.process(&mut buf), Ok(Some(expect.clone())));
        assert_eq!(buf.remaining(), 2);

        // test decoding of cut-off frame / data loss (with data being cut off)
        let mut buf = BytesMut::new();
        buf.put_slice(&data[..10]);
        buf.put_slice(&data[..]);

        assert_eq!(dec.process(&mut buf), Err(Error::InvalidChecksum));
        assert_eq!(dec.process(&mut buf), Ok(Some(expect.clone())));
        assert_eq!(buf.remaining(), 2);

        // test frame flag as escaped byte
        let mut buf = BytesMut::from(&data[..10]);
        buf.put_slice(&data[..]);
        buf[9] = consts::flags::FRAME;

        assert_eq!(dec.process(&mut buf), Err(Error::UnexpectedEndOfFrame));
        assert_eq!(dec.process(&mut buf), Err(Error::UnexpectedData));
        assert_eq!(dec.process(&mut buf), Ok(Some(expect.clone())));
        assert_eq!(buf.remaining(), 2);

    }
}
