use bytes::{BufMut, BytesMut};

use super::{consts, crc::Crc32, varint, Frame};


struct ByteEscape<B: BufMut> {
    buf: B,
}

impl<B: BufMut> ByteEscape<B> {
    fn new(buf: B) -> Self {
        Self { buf }
    }

    fn put_u8(&mut self, byte: u8) {
        match byte {
            consts::flags::ESCAPE | consts::flags::FRAME => self.buf.put_slice(&[
                consts::flags::ESCAPE,
                consts::escape::MASK ^ byte
            ]),
            _ => self.buf.put_u8(byte),
        }
    }

    fn put_frame_flag(&mut self) {
        self.buf.put_u8(super::consts::flags::FRAME)
    }
}

impl ByteEscape<&mut BytesMut> {
    fn reserve(&mut self, additional: usize) -> &mut Self {
        self.buf.reserve(additional);
        self
    }
}


struct Encoder<B: BufMut> {
    buf: ByteEscape<B>,
    crc: Crc32,
}

impl<B: BufMut> Encoder<B> {
    fn new(buf: B) -> Self {
        Self {
            buf: ByteEscape::new(buf),
            crc: Crc32::new(),
        }
    }

    fn flag(&mut self) -> &mut Self {
        self.buf.put_frame_flag();
        self
    }

    fn put_u8(&mut self, byte: u8) -> &mut Self {
        self.crc.put_u8(byte);
        self.buf.put_u8(byte);
        self
    }

    fn put_bytes<T: IntoIterator<Item = u8>>(&mut self, bytes: T) -> &mut Self {
        for b in bytes.into_iter() {
            self.put_u8(b);
        }
        self
    }

    fn finalize(&mut self) {
        self.put_bytes(self.crc.value().to_le_bytes());
        self.flag();
    }
}

impl Encoder<&mut BytesMut> {
    fn reserve(&mut self, additional: usize) -> &mut Self {
        self.buf.reserve(additional);
        self
    }
}


pub fn encode(buf: &mut BytesMut, frame: &Frame) {
    Encoder::new(buf)
        .reserve(frame.data.len() + 8)              // reserve at least data-size + min-frame-size
        .flag()                                     // flag
        .put_bytes(varint::encode(frame.address))   // address
        .put_u8(frame.control)                      // control
        .put_bytes(frame.data.iter().copied())      // data
        .reserve(5)                                 // reserve CRC32 + flag
        .finalize()                                 // checksum and flag
}

pub fn encode_bytes(frame: &Frame) -> BytesMut {
    let mut buf = BytesMut::new();
    encode(&mut buf, frame);
    buf
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_escape_bytes() {
        fn e(src: &[u8]) -> Vec<u8> {
            let mut dst = Vec::new();
            let mut buf = ByteEscape::new(&mut dst);

            for byte in src {
                buf.put_u8(*byte);
            }

            dst
        }

        assert_eq!(e(&[0x00, 0x00]), [0x00, 0x00]);
        assert_eq!(e(&[0x7D]), [0x7D, 0x5D]);
        assert_eq!(e(&[0x7E]), [0x7D, 0x5E]);
        assert_eq!(e(&[0x01, 0x7D, 0x02]), [0x01, 0x7D, 0x5D, 0x02]);
        assert_eq!(e(&[0x01, 0x7E, 0x02]), [0x01, 0x7D, 0x5E, 0x02]);
        assert_eq!(e(&[0x7D, 0x7E]), [0x7D, 0x5D, 0x7D, 0x5E]);
        assert_eq!(e(&[0x7F, 0x5D, 0x7E]), [0x7F, 0x5D, 0x7D, 0x5E]);
    }

    #[test]
    fn test_encode() {
        assert_eq!([
            0x7e, 0x06, 0x08, 0x09, 0x03, 0x8b, 0x3b, 0xf7, 0x42, 0x7e,
        ], &encode_bytes(&Frame {
            address: 0x010203,
            control: 0x03,
            data: vec![].into(),
        })[..]);

        assert_eq!([
            0x7e, 0x06, 0x08, 0x09, 0x03, 0x05, 0x06, 0x07, 0x7d, 0x5d,
            0x7d, 0x5e, 0x7f, 0xff, 0xe6, 0x2d, 0x17, 0xc6, 0x7e,
        ], &encode_bytes(&Frame {
            address: 0x010203,
            control: 0x03,
            data: vec![0x05, 0x06, 0x07, 0x7d, 0x7e, 0x7f, 0xff].into(),
        })[..]);
    }
}
