use crate::pwrpc::Error;
use crate::pwrpc::client::{Client, Request, UnaryResponse, ClientHandle};
use crate::pwrpc::id::PathRef;
use crate::pwrpc::types::RpcPacket;

use super::addr;
use super::addr::Peer;
use super::types::SoftwareInfo;


pub async fn resolve_channel<S, E>(client: &mut Client<S>) -> Result<u32, Error>
where
    S: futures::Sink<RpcPacket>,
    S: futures::Stream<Item = Result<RpcPacket, E>> + Unpin,
    Error: From<E>,
    Error: From<S::Error>,
{
    tracing::trace!("resolving channel");

    let channels = (
        addr::channel_id(Peer::MaestroA, Peer::Case).unwrap(),
        addr::channel_id(Peer::MaestroA, Peer::LeftBtCore).unwrap(),
        addr::channel_id(Peer::MaestroA, Peer::RightBtCore).unwrap(),
        addr::channel_id(Peer::MaestroB, Peer::Case).unwrap(),
        addr::channel_id(Peer::MaestroB, Peer::LeftBtCore).unwrap(),
        addr::channel_id(Peer::MaestroB, Peer::RightBtCore).unwrap(),
    );

    let tasks = (
        try_open_channel(client.handle(), channels.0),
        try_open_channel(client.handle(), channels.1),
        try_open_channel(client.handle(), channels.2),
        try_open_channel(client.handle(), channels.3),
        try_open_channel(client.handle(), channels.4),
        try_open_channel(client.handle(), channels.5),
    );

    let channel = tokio::select! {
        // Ensure that the open() calls are registered before we start running
        // the client.
        biased;

        res = tasks.0 => { res? },
        res = tasks.1 => { res? },
        res = tasks.2 => { res? },
        res = tasks.3 => { res? },
        res = tasks.4 => { res? },
        res = tasks.5 => { res? },
        res = client.run() => { res?; return Err(Error::aborted("client terminated")) }
    };

    tracing::trace!(channel=channel, "channel resolved");
    Ok(channel)
}

async fn try_open_channel(mut handle: ClientHandle, channel_id: u32) -> Result<u32, Error> {
    let path = PathRef::new("maestro_pw.Maestro/GetSoftwareInfo");
    let service_id = path.service().hash();
    let method_id = path.method().hash();

    let req = Request {
        channel_id,
        service_id,
        method_id,
        call_id: 0xffffffff,
        message: (),
    };

    let mut rsp: UnaryResponse<SoftwareInfo> = handle.open_unary(req)?;

    rsp.result().await?;
    Ok(channel_id)
}
