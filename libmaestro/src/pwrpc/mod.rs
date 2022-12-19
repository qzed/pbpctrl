pub mod codec;

pub mod packet {
    include!(concat!(env!("OUT_DIR"), "/pw.rpc.packet.rs"));
}
