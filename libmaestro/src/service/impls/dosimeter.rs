use crate::protocol::types::{
    DosimeterSummary, DosimeterLiveDbMsg,
};
use crate::pwrpc::client::{ClientHandle, ServerStreamRpc, StreamResponse, UnaryRpc};
use crate::pwrpc::Error;


#[derive(Debug, Clone)]
pub struct DosimeterService {
    client: ClientHandle,
    channel_id: u32,

    rpc_fetch_daily_summaries: UnaryRpc<(), DosimeterSummary>,
    rpc_sub_live_db: ServerStreamRpc<(), DosimeterLiveDbMsg>,
}

impl DosimeterService {
    pub fn new(client: ClientHandle, channel_id: u32) -> Self {
        Self {
            client,
            channel_id,

            rpc_fetch_daily_summaries: UnaryRpc::new("maestro_pw.Dosimeter/FetchDailySummaries"),
            rpc_sub_live_db: ServerStreamRpc::new("maestro_pw.Dosimeter/SubscribeToLiveDb"),
        }
    }

    pub async fn fetch_daily_summaries(&mut self) -> Result<DosimeterSummary, Error> {
        self.rpc_fetch_daily_summaries.call(&mut self.client, self.channel_id, 0, ())?
            .result().await
    }

    pub fn subscribe_to_live_db(&mut self) -> Result<StreamResponse<DosimeterLiveDbMsg>, Error> {
        self.rpc_sub_live_db.call(&mut self.client, self.channel_id, 0, ())
    }
}
