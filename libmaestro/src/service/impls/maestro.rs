use crate::protocol::types::{
    self, read_setting_msg, settings_rsp, write_setting_msg, HardwareInfo, OobeActionRsp,
    ReadSettingMsg, RuntimeInfo, SettingsRsp, SoftwareInfo, WriteSettingMsg,
};
use crate::pwrpc::client::{ClientHandle, ServerStreamRpc, StreamResponse, UnaryRpc};
use crate::pwrpc::Error;
use crate::service::settings::{Setting, SettingId, SettingValue};


#[derive(Debug, Clone)]
pub struct MaestroService {
    client: ClientHandle,
    channel_id: u32,

    rpc_get_software_info: UnaryRpc<(), SoftwareInfo>,
    rpc_get_hardware_info: UnaryRpc<(), HardwareInfo>,
    rpc_sub_runtime_info: ServerStreamRpc<(), RuntimeInfo>,

    rpc_write_setting: UnaryRpc<WriteSettingMsg, ()>,
    rpc_read_setting: UnaryRpc<ReadSettingMsg, SettingsRsp>,
    rpc_sub_settings_changes: ServerStreamRpc<(), SettingsRsp>,

    rpc_sub_oobe_actions: ServerStreamRpc<(), OobeActionRsp>,
}

impl MaestroService {
    pub fn new(client: ClientHandle, channel_id: u32) -> Self {
        Self {
            client,
            channel_id,

            rpc_get_software_info: UnaryRpc::new("maestro_pw.Maestro.GetSoftwareInfo"),
            rpc_get_hardware_info: UnaryRpc::new("maestro_pw.Maestro.GetHardwareInfo"),
            rpc_sub_runtime_info: ServerStreamRpc::new("maestro_pw.Maestro.SubscribeRuntimeInfo"),

            rpc_write_setting: UnaryRpc::new("maestro_pw.Maestro.WriteSetting"),
            rpc_read_setting: UnaryRpc::new("maestro_pw.Maestro.ReadSetting"),
            rpc_sub_settings_changes: ServerStreamRpc::new("maestro_pw.Maestro.SubscribeToSettingsChanges"),

            rpc_sub_oobe_actions: ServerStreamRpc::new("maestro_pw.Maestro.SubscribeToOobeActions"),
        }
    }

    pub async fn get_software_info(&mut self) -> Result<SoftwareInfo, Error> {
        self.rpc_get_software_info.call(&mut self.client, self.channel_id, 0, ()).await?
            .result().await
    }

    pub async fn get_hardware_info(&mut self) -> Result<HardwareInfo, Error> {
        self.rpc_get_hardware_info.call(&mut self.client, self.channel_id, 0, ()).await?
            .result().await
    }

    pub async fn subscribe_to_runtime_info(&mut self) -> Result<StreamResponse<RuntimeInfo>, Error> {
        self.rpc_sub_runtime_info.call(&mut self.client, self.channel_id, 0, ()).await
    }

    pub async fn write_setting_raw(&mut self, setting: WriteSettingMsg) -> Result<(), Error> {
        self.rpc_write_setting.call(&mut self.client, self.channel_id, 0, setting).await?
            .result().await
    }

    pub async fn write_setting(&mut self, setting: SettingValue) -> Result<(), Error> {
        let setting = types::SettingValue {
            value_oneof: Some(setting.into()),
        };

        let setting = WriteSettingMsg {
            value_oneof: Some(write_setting_msg::ValueOneof::Setting(setting)),
        };

        self.write_setting_raw(setting).await
    }

    pub async fn read_setting_raw(&mut self, setting: ReadSettingMsg) -> Result<SettingsRsp, Error> {
        self.rpc_read_setting.call(&mut self.client, self.channel_id, 0, setting).await?
            .result().await
    }

    pub async fn read_setting_var(&mut self, setting: SettingId) -> Result<SettingValue, Error> {
        let setting = read_setting_msg::ValueOneof::SettingsId(setting.into());
        let setting = ReadSettingMsg { value_oneof: Some(setting) };

        let value = self.read_setting_raw(setting).await?;

        let value = value.value_oneof
            .ok_or_else(|| Error::invalid_argument("did not receive any settings value"))?;

        let settings_rsp::ValueOneof::Value(value) = value;

        let value = value.value_oneof
            .ok_or_else(|| Error::invalid_argument("did not receive any settings value"))?;

        Ok(value.into())
    }

    pub async fn read_setting<T>(&mut self, setting: T) -> Result<T::Type, Error>
    where
        T: Setting,
    {
        let value = self.read_setting_var(setting.id()).await?;

        T::from_var(value)
            .ok_or_else(|| Error::invalid_argument("failed to decode settings value"))
    }

    pub async fn subscribe_to_settings_changes(&mut self) -> Result<StreamResponse<SettingsRsp>, Error> {
        self.rpc_sub_settings_changes.call(&mut self.client, self.channel_id, 0, ()).await
    }

    pub async fn subscribe_to_oobe_actions(&mut self) -> Result<StreamResponse<OobeActionRsp>, Error> {
        self.rpc_sub_oobe_actions.call(&mut self.client, self.channel_id, 0, ()).await
    }

    // TODO:
    // - SetWallClock
}
