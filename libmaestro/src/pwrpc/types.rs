#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RpcType {
    Unary,
    ServerStream,
    ClientStream,
    BidirectionalStream,
}

impl RpcType {
    pub fn has_server_stream(&self) -> bool {
        match *self {
            RpcType::ServerStream | RpcType::BidirectionalStream => true,
            RpcType::Unary | RpcType::ClientStream => false,
        }
    }

    pub fn has_client_stream(&self) -> bool {
        match *self {
            RpcType::ClientStream | RpcType::BidirectionalStream => true,
            RpcType::Unary | RpcType::ServerStream => false,
        }
    }
}


mod generated {
    include!(concat!(env!("OUT_DIR"), "/pw.rpc.packet.rs"));
}

pub use generated::PacketType;
pub use generated::RpcPacket;
