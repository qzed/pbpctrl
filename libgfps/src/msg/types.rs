//! RFCOMM events and event-related enums.

use num_enum::{IntoPrimitive, FromPrimitive};


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub group: u8,
    pub code: u8,
    pub data: Box<[u8]>,
}


#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
pub enum EventGroup {
    Bluetooth = 0x01,
    Logging = 0x02,
    Device = 0x03,
    DeviceAction = 0x04,
    DeviceConfiguration = 0x05,
    DeviceCapabilitySync = 0x06,
    SmartAudioSourceSwitching = 0x07,
    Acknowledgement = 0xff,

    #[num_enum(catch_all)]
    Unknown(u8) = 0xfe,
}

#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
pub enum BluetoothEventCode {
    EnableSilenceMode = 0x01,
    DisableSilenceMode = 0x02,

    #[num_enum(catch_all)]
    Unknown(u8),
}

#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
pub enum LoggingEventCode {
    LogFull = 0x01,
    LogSaveToBuffer = 0x02,

    #[num_enum(catch_all)]
    Unknown(u8),
}

#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
pub enum DeviceEventCode {
    ModelId = 0x01,
    BleAddress = 0x02,
    BatteryInfo = 0x03,
    ActiveComponentsRequest = 0x05,
    ActiveComponentsResponse = 0x06,
    Capability = 0x07,
    PlatformType = 0x08,
    FirmwareVersion = 0x09,
    SectionNonce = 0x0a,

    #[num_enum(catch_all)]
    Unknown(u8),
}

#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
pub enum DeviceActionEventCode {
    DeviceActionRing = 0x01,

    #[num_enum(catch_all)]
    Unknown(u8),
}

#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
pub enum DeviceConfigurationEventCode {
    BufferSize = 0x01,

    #[num_enum(catch_all)]
    Unknown(u8),
}

#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
pub enum DeviceCapabilitySyncEventCode {
    CapabilityUpdate = 0x01,
    ConfigurableBufferSizeRange = 0x02,

    #[num_enum(catch_all)]
    Unknown(u8),
}

#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
pub enum AcknowledgementEventCode {
    Ack = 0x01,
    Nak = 0x02,

    #[num_enum(catch_all)]
    Unknown(u8),
}

#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
pub enum PlatformType {
    Android = 0x01,

    #[num_enum(catch_all)]
    Unknown(u8),
}

#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, FromPrimitive)]
pub enum SassEventCode {
    GetCapabilityOfSass = 0x10,
    NotifyCapabilityOfSass = 0x11,
    SetMultiPointState = 0x12,
    SwitchAudioSourceBetweenConnectedDevices = 0x30,
    SwitchBack = 0x31,
    NotifyMultiPointSwitchEvent = 0x32,
    GetConnectionStatus = 0x33,
    NotifyConnectionStatus = 0x34,
    SassInitiatedConnection = 0x40,
    IndicateInUseAccountKey = 0x41,
    SetCustomData = 0x42,

    #[num_enum(catch_all)]
    Unknown(u8),
}
