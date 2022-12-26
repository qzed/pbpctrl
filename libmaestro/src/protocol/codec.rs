use bytes::BytesMut;

use prost::Message;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Decoder, Framed, Encoder};

use crate::pwrpc::types::RpcPacket;
use crate::hdlc;

use super::addr;


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
    type Item = RpcPacket;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        match self.hdlc.decode(src)? {
            Some(frame) => {
                if frame.control != 0x03 {
                    tracing::warn!("unexpected control type: {}", frame.control);
                    return Ok(None);
                }

                let packet = RpcPacket::decode(&frame.data[..])?;
                Ok(Some(packet))
            }
            None => Ok(None),
        }
    }
}

impl Encoder<&RpcPacket> for Codec {
    type Error = std::io::Error;

    fn encode(&mut self, packet: &RpcPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let address = addr::address_for_channel(packet.channel_id).unwrap();

        let frame = hdlc::Frame {
            address: address.value(),
            control: 0x03,
            data: packet.encode_to_vec().into(),    // TODO: can we avoid these allocations?
        };

        self.hdlc.encode(&frame, dst)
    }
}

impl Encoder<RpcPacket> for Codec {
    type Error = std::io::Error;

    fn encode(&mut self, packet: RpcPacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        self.encode(&packet, dst)
    }
}
