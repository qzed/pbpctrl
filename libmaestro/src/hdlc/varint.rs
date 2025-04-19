//! Support for variable length integer encoding as used in the HDLC frame encoding.

use arrayvec::ArrayVec;


pub fn decode<'a, S: IntoIterator<Item = &'a u8>>(src: S) -> Result<(u32, usize), DecodeError> {
    let mut address = 0;

    for (i, b) in src.into_iter().copied().enumerate() {
        address |= ((b >> 1) as u64) << (i * 7);

        if address > u32::MAX as u64 {
            Err(DecodeError::Overflow)?;
        }

        if b & 0x01 == 0x01 {
            return Ok((address as u32, i + 1));
        }
    }

    Err(DecodeError::Incomplete)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    Incomplete,
    Overflow,
}


pub fn encode(num: u32) -> Encode {
    Encode { num, done: false }
}

pub fn encode_vec(num: u32) -> ArrayVec<u8, { num_bytes(u32::MAX) }> {
    encode(num).collect()
}

pub struct Encode {
    num: u32,
    done: bool,
}

impl Iterator for Encode {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        if (self.num >> 7) != 0 {
            let b = ((self.num & 0x7F) as u8) << 1;
            self.num >>= 7;

            Some(b)
        } else if !self.done {
            let b = (((self.num & 0x7F) as u8) << 1) | 1;
            self.done = true;

            Some(b)
        } else {
            None
        }
    }
}


pub const fn num_bytes(value: u32) -> usize {
    if value == 0 {
        1
    } else {
        (u32::BITS - value.leading_zeros()).div_ceil(7)
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_decode() {
        assert_eq!(decode(&[0x01]).unwrap(), (0x00, 1));
        assert_eq!(decode(&[0x00, 0x00, 0x00, 0x01]).unwrap(), (0x00, 4));
        assert_eq!(decode(&[0x11, 0x00]).unwrap(), (0x0008, 1));
        assert_eq!(decode(&[0x10, 0x21]).unwrap(), (0x0808, 2));

        assert_eq!(decode(&[0x01]).unwrap(), (0x00, 1));
        assert_eq!(decode(&[0x03]).unwrap(), (0x01, 1));

        assert_eq!(decode(&[0xff]).unwrap(), (0x7f, 1));
        assert_eq!(decode(&[0x00, 0x03]).unwrap(), (0x80, 2));

        assert_eq!(decode(&[0xfe, 0xff]).unwrap(), (0x3fff, 2));
        assert_eq!(decode(&[0x00, 0x00, 0x03]).unwrap(), (0x4000, 3));

        assert_eq!(decode(&[0xfe, 0xfe, 0xff]).unwrap(), (0x1f_ffff, 3));
        assert_eq!(decode(&[0x00, 0x00, 0x00, 0x03]).unwrap(), (0x20_0000, 4));

        assert_eq!(decode(&[0xfe, 0xfe, 0xfe, 0xff]).unwrap(), (0x0fff_ffff, 4));
        assert_eq!(decode(&[0x00, 0x00, 0x00, 0x00, 0x03]).unwrap(), (0x1000_0000, 5));

        assert_eq!(decode(&[0xfe, 0x03]).unwrap(), (u8::MAX as _, 2));
        assert_eq!(decode(&[0xfe, 0xfe, 0x07]).unwrap(), (u16::MAX as _, 3));
        assert_eq!(decode(&[0xfe, 0xfe, 0xfe, 0xfe, 0x1f]).unwrap(), (u32::MAX, 5));

        assert_eq!(decode(&[0xFE]), Err(DecodeError::Incomplete));
        assert_eq!(decode(&[0xFE, 0xFE, 0xFE, 0xFE, 0xFF]), Err(DecodeError::Overflow));
    }

    #[test]
    fn test_encode() {
        assert_eq!(encode_vec(0x01234)[..], [0x68, 0x49]);
        assert_eq!(encode_vec(0x87654)[..], [0xa8, 0xd8, 0x43]);

        assert_eq!(encode_vec(0x00)[..], [0x01]);
        assert_eq!(encode_vec(0x01)[..], [0x03]);

        assert_eq!(encode_vec(0x7f)[..], [0xff]);
        assert_eq!(encode_vec(0x80)[..], [0x00, 0x03]);

        assert_eq!(encode_vec(0x3fff)[..], [0xfe, 0xff]);
        assert_eq!(encode_vec(0x4000)[..], [0x00, 0x00, 0x03]);

        assert_eq!(encode_vec(0x1f_ffff)[..], [0xfe, 0xfe, 0xff]);
        assert_eq!(encode_vec(0x20_0000)[..], [0x00, 0x00, 0x00, 0x03]);

        assert_eq!(encode_vec(0x0fff_ffff)[..], [0xfe, 0xfe, 0xfe, 0xff]);
        assert_eq!(encode_vec(0x1000_0000)[..], [0x00, 0x00, 0x00, 0x00, 0x03]);

        assert_eq!(encode_vec(u8::MAX as _)[..], [0xfe, 0x03]);
        assert_eq!(encode_vec(u16::MAX as _)[..], [0xfe, 0xfe, 0x07]);
        assert_eq!(encode_vec(u32::MAX)[..], [0xfe, 0xfe, 0xfe, 0xfe, 0x1f]);
    }

    #[test]
    fn test_num_bytes() {
        assert_eq!(num_bytes(0x00), 1);
        assert_eq!(num_bytes(0x01), 1);

        assert_eq!(num_bytes(0x7f), 1);
        assert_eq!(num_bytes(0x80), 2);

        assert_eq!(num_bytes(0x3fff), 2);
        assert_eq!(num_bytes(0x4000), 3);

        assert_eq!(num_bytes(0x1f_ffff), 3);
        assert_eq!(num_bytes(0x20_0000), 4);

        assert_eq!(num_bytes(0x0fff_ffff), 4);
        assert_eq!(num_bytes(0x1000_0000), 5);

        assert_eq!(num_bytes(u8::MAX as _), 2);
        assert_eq!(num_bytes(u16::MAX as _), 3);
        assert_eq!(num_bytes(u32::MAX), 5);
    }
}
