use crate::protocol::types::QuietModeStatusEvent;
use crate::pwrpc::client::{ClientHandle, ServerStreamRpc, StreamResponse};
use crate::pwrpc::types::RpcPacket;
use crate::pwrpc::Error;


pub struct MultipointService<S> {
    client: ClientHandle<S>,
    channel_id: u32,

    rpc_sub_quiet_mode_status: ServerStreamRpc<(), QuietModeStatusEvent>,
}

impl<S, E> MultipointService<S>
where
    S: futures::Sink<RpcPacket>,
    S: futures::Stream<Item = Result<RpcPacket, E>>,
    S: Unpin,
    Error: From<S::Error>,
{
    pub fn new(client: ClientHandle<S>, channel_id: u32) -> Self {
        Self {
            client,
            channel_id,

            rpc_sub_quiet_mode_status: ServerStreamRpc::new("maestro_pw.Multipoint.SubscribeToQuietModeStatus"),
        }
    }

    pub async fn subscribe_to_quiet_mode_status(&self) -> Result<StreamResponse<QuietModeStatusEvent>, Error> {
        self.rpc_sub_quiet_mode_status.call(&self.client, self.channel_id, 0, ()).await
    }

    // TODO:
    // - ForceMultipointSwitch
}
