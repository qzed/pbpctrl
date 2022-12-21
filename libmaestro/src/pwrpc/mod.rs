pub mod codec;
pub mod id;
pub mod status;

pub mod packet {
    include!(concat!(env!("OUT_DIR"), "/pw.rpc.packet.rs"));
}
