use crate::protocol::types::QuietModeStatusEvent;
use crate::pwrpc::client::{ClientHandle, ServerStreamRpc, StreamResponse};
use crate::pwrpc::Error;


#[derive(Debug, Clone)]
pub struct MultipointService {
    client: ClientHandle,
    channel_id: u32,

    rpc_sub_quiet_mode_status: ServerStreamRpc<(), QuietModeStatusEvent>,
}

impl MultipointService {
    pub fn new(client: ClientHandle, channel_id: u32) -> Self {
        Self {
            client,
            channel_id,

            rpc_sub_quiet_mode_status: ServerStreamRpc::new("maestro_pw.Multipoint/SubscribeToQuietModeStatus"),
        }
    }

    pub async fn subscribe_to_quiet_mode_status(&mut self) -> Result<StreamResponse<QuietModeStatusEvent>, Error> {
        self.rpc_sub_quiet_mode_status.call(&mut self.client, self.channel_id, 0, ())
    }

    // TODO:
    // - ForceMultipointSwitch
}
