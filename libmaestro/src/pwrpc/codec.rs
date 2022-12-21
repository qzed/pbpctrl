use bytes::BytesMut;

use prost::Message;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Decoder, Framed, Encoder};

use super::packet::RpcPacket;
use crate::hdlc;



#[derive(Debug, Clone, PartialEq)]
pub struct Packet {
    pub address: u32,
    pub rpc: RpcPacket,
}

pub struct Codec {
    hdlc: hdlc::Codec,
}

impl Codec {
    pub fn new() -> Self {
        Self {
            hdlc: hdlc::Codec::new(),
        }
    }

    pub fn wrap<T>(self, io: T) -> Framed<T, Codec>
    where
        T: AsyncRead + AsyncWrite,
    {
        Framed::with_capacity(io, self, 4096 as _)
    }
}

impl Default for Codec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for Codec {
    type Item = Packet;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.hdlc.decode(src)? {
            Some(frame) => {
                if frame.control != 0x03 {
                    log::warn!(target: "pwrpc:decoder", "unexpected control type: {}", frame.control);
                    return Ok(None);
                }

                let rpc = RpcPacket::decode(&frame.data[..])?;
                let packet = Packet { address: frame.address, rpc };

                Ok(Some(packet))
            }
            None => Ok(None),
        }
    }
}

impl Encoder<&Packet> for Codec {
    type Error = std::io::Error;

    fn encode(&mut self, packet: &Packet, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let frame = hdlc::Frame {
            address: packet.address,
            control: 0x03,
            data: packet.rpc.encode_to_vec().into(),    // TODO: can we avoid these allocations?
        };

        self.hdlc.encode(&frame, dst)
    }
}

impl Encoder<Packet> for Codec {
    type Error = std::io::Error;

    fn encode(&mut self, packet: Packet, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.encode(&packet, dst)
    }
}
