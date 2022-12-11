//! Simple example for listening to GFPS messages sent via the RFCOMM channel.
//!
//! Usage:
//!   cargo run --example listen -- <bluetooth-device-address>

use std::collections::HashSet;
use std::str::FromStr;

use bluer::{Address, Session};
use bluer::rfcomm::{Profile, ReqError, Role};

use futures::StreamExt;

use gfps::msg::{
    AcknowledgementEventCode, Codec, DeviceActionEventCode, DeviceCapabilitySyncEventCode,
    DeviceConfigurationEventCode, DeviceEventCode, EventGroup, Message, PlatformType,
    SassEventCode, LoggingEventCode, BluetoothEventCode,
};

use num_enum::FromPrimitive;


#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    // handle command line arguments
    let addr = std::env::args().skip(1).next().expect("need device address as argument");
    let addr = Address::from_str(&addr)?;

    // set up session
    let session = Session::new().await?;
    let adapter = session.default_adapter().await?;

    println!("Using adapter '{}'", adapter.name());

    // get device
    let dev = adapter.device(addr)?;
    let uuids = {
        let mut uuids = Vec::from_iter(dev.uuids().await?
            .unwrap_or_else(HashSet::new)
            .into_iter());

        uuids.sort_unstable();
        uuids
    };

    println!("Found device:");
    println!("  alias:     {}", dev.alias().await?);
    println!("  address:   {}", dev.address());
    println!("  paired:    {}", dev.is_paired().await?);
    println!("  connected: {}", dev.is_connected().await?);
    println!("  UUIDs:");
    for uuid in uuids {
        println!("    {}", uuid);
    }
    println!();

    // try to reconnect if connection is reset
    loop {
        let stream = {
            // register GFPS profile
            println!("Registering GFPS profile...");

            let profile = Profile {
                uuid: gfps::msg::UUID,
                role: Some(Role::Client),
                require_authentication: Some(false),
                require_authorization: Some(false),
                auto_connect: Some(false),
                ..Default::default()
            };

            let mut profile_handle = session.register_profile(profile).await?;

            // connect profile
            println!("Connecting GFPS profile...");

            loop {
                tokio::select! {
                    res = async {
                        let _ = dev.connect().await;
                        dev.connect_profile(&gfps::msg::UUID).await
                    } => {
                        if let Err(err) = res {
                            println!("Connecting GFPS profile failed: {:?}", err);
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
                    },
                    req = profile_handle.next() => {
                        let req = req.expect("no connection request received");

                        if req.device() == addr {
                            println!("Accepting request...");
                            break req.accept()?;
                        } else {
                            println!("Rejecting unknown device {}", req.device());
                            req.reject(ReqError::Rejected);
                        }
                    },
                }
            }
        };

        println!("Profile connected");

        // listen to event messages
        let codec = Codec::new();
        let mut stream = codec.wrap(stream);

        println!("Listening...");
        println!();

        while let Some(msg) = stream.next().await {
            match msg {
                Ok(msg) => {
                    print_message(&msg);
                }
                Err(e) if e.raw_os_error() == Some(104) => {
                    // The Pixel Buds Pro can hand off processing between each
                    // other. On a switch, the connection is reset. Wait a bit
                    // and then try to reconnect.
                    println!();
                    println!("Connection reset. Attempting to reconnect...");
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    break;
                }
                Err(e) => {
                    Err(e)?;
                }
            }
        }
    }
}

fn print_message(msg: &Message) {
    let group = EventGroup::from_primitive(msg.group);

    match group {
        EventGroup::Bluetooth => {
            let code = BluetoothEventCode::from_primitive(msg.code);

            println!("Bluetooth (0x{:02X}) :: ", msg.group);

            match code {
                BluetoothEventCode::EnableSilenceMode => {
                    println!("Enable Silence Mode (0x{:02X})", msg.code);
                },
                BluetoothEventCode::DisableSilenceMode => {
                    println!("Disable Silence Mode (0x{:02X})", msg.code);
                },
                BluetoothEventCode::Unknown(_) | _ => {
                    println!("Unknown (0x{:02X})", msg.code);
                }
            }

            print_message_body_unknown(msg);
            println!();
        }
        EventGroup::Logging => {
            let code = LoggingEventCode::from_primitive(msg.code);

            println!("Companion App (0x{:02X}) :: ", msg.group);

            match code {
                LoggingEventCode::LogFull => {
                    println!("Log Full (0x{:02X})", msg.code);
                }
                LoggingEventCode::LogSaveToBuffer => {
                    println!("Log Save Buffer (0x{:02X})", msg.code);
                }
                LoggingEventCode::Unknown(_) | _ => {
                    println!("Unknown (0x{:02X})", msg.code);
                }
            }

            print_message_body_unknown(msg);
            println!();
        }
        EventGroup::Device => {
            let code = DeviceEventCode::from_primitive(msg.code);

            print!("Device Information (0x{:02X}) :: ", msg.group);

            match code {
                DeviceEventCode::ModelId => {
                    println!("Model Id (0x{:02X})", msg.code);
                    println!("  model: {:02X}{:02X}{:02X}", msg.data[0], msg.data[1], msg.data[2]);
                }
                DeviceEventCode::BleAddress => {
                    println!("BLE Address (0x{:02X})", msg.code);
                    println!("  address: {}", Address::new(msg.data[0..6].try_into().unwrap()));
                }
                DeviceEventCode::BatteryInfo => {
                    println!("Battery Info (0x{:02X})", msg.code);

                    let left_level = msg.data[0] & 0x7F;
                    let left_status = if msg.data[0] & 0x80 != 0 {
                        "charging"
                    } else {
                        "not charging"
                    };

                    let right_level = msg.data[1] & 0x7F;
                    let right_status = if msg.data[1] & 0x80 != 0 {
                        "charging"
                    } else {
                        "not charging"
                    };

                    let case_level = msg.data[2] & 0x7F;
                    let case_status = if msg.data[2] & 0x80 != 0 {
                        "charging"
                    } else {
                        "not charging"
                    };

                    if left_level == 0x7F {
                        println!("  left bud:  unknown");
                    } else {
                        println!("  left bud:  {}% ({})", left_level, left_status);
                    }

                    if right_level == 0x7F {
                        println!("  right bud: unknown");
                    } else {
                        println!("  right bud: {}% ({})", right_level, right_status);
                    }

                    if case_level == 0x7F {
                        println!("  case bud:  unknown");
                    } else {
                        println!("  case bud:  {}% ({})", case_level, case_status);
                    }
                }
                DeviceEventCode::BatteryTime => {
                    println!("Remaining Battery Time (0x{:02X})", msg.code);

                    let time = match msg.data.len() {
                        1 => msg.data[0] as u16,
                        2 => u16::from_be_bytes(msg.data[0..2].try_into().unwrap()),
                        _ => panic!("invalid format"),
                    };

                    println!("  time: {} minutes", time);
                }
                DeviceEventCode::ActiveComponentsRequest => {
                    println!("Active Components Request (0x{:02X})", msg.code);
                }
                DeviceEventCode::ActiveComponentsResponse => {
                    println!("Active Components Response (0x{:02X})", msg.code);
                    println!("  components: {:08b}", msg.data[0]);
                }
                DeviceEventCode::Capability => {
                    println!("Capability (0x{:02X})", msg.code);
                    println!("  capabilities: {:08b}", msg.data[0]);
                }
                DeviceEventCode::PlatformType => {
                    println!("Platform Type (0x{:02X})", msg.code);

                    let platform = PlatformType::from_primitive(msg.data[0]);
                    match platform {
                        PlatformType::Android => {
                            println!("  platform: Android (0x{:02X})", msg.data[0]);
                            println!("  SDK version: {:02X?})", msg.data[1]);
                        }
                        PlatformType::Unknown(_) | _ => {
                            println!("  platform: Unknown (0x{:02X})", msg.data[0]);
                            println!("  platform data: 0x{:02X?})", msg.data[1]);
                        }
                    }
                }
                DeviceEventCode::FirmwareVersion => {
                    println!("Firmware Version (0x{:02X})", msg.code);
                    if let Ok(ver) = std::str::from_utf8(&msg.data) {
                        println!("  version: {:?}", ver);
                    } else {
                        println!("  version: {:02X?}", msg.data);
                    }
                }
                DeviceEventCode::SectionNonce => {
                    println!("Session Nonce (0x{:02X})", msg.code);
                    println!("  nonce: {:02X?}", msg.data);
                }
                DeviceEventCode::Unknown(_) | _ => {
                    println!("Unknown (0x{:02X})", msg.code);
                    print_message_body_unknown(msg);
                }
            }

            println!();
        }
        EventGroup::DeviceAction => {
            let code = DeviceActionEventCode::from_primitive(msg.code);

            print!("Device Action (0x{:02X}) :: ", msg.group);

            match code {
                DeviceActionEventCode::Ring => {
                    println!("Ring (0x{:02X})", msg.code);
                }
                DeviceActionEventCode::Unknown(_) | _ => {
                    println!("Unknown (0x{:02X})", msg.code);
                }
            }

            print_message_body_unknown(msg);
            println!();
        }
        EventGroup::DeviceConfiguration => {
            let code = DeviceConfigurationEventCode::from_primitive(msg.code);

            print!("Device Configuration (0x{:02X}) :: ", msg.group);

            match code {
                DeviceConfigurationEventCode::BufferSize => {
                    println!("Buffer Size (0x{:02X})", msg.code);
                }
                DeviceConfigurationEventCode::Unknown(_) | _ => {
                    println!("Unknown (0x{:02X})", msg.code);
                }
            }

            print_message_body_unknown(msg);
            println!();
        }
        EventGroup::DeviceCapabilitySync => {
            let code = DeviceCapabilitySyncEventCode::from_primitive(msg.code);

            print!("Device Cpabilities Sync (0x{:02X}) :: ", msg.group);

            match code {
                DeviceCapabilitySyncEventCode::CapabilityUpdate => {
                    println!("Capability Update (0x{:02X})", msg.code);
                }
                DeviceCapabilitySyncEventCode::ConfigurableBufferSizeRange => {
                    println!("Configurable Buffer Size Range (0x{:02X})", msg.code);
                }
                DeviceCapabilitySyncEventCode::Unknown(_) | _ => {
                    println!("Unknown (0x{:02X})", msg.code);
                }
            }

            print_message_body_unknown(msg);
            println!();
        }
        EventGroup::SmartAudioSourceSwitching => {
            let code = SassEventCode::from_primitive(msg.code);

            print!("Smart Audio Source Switching (0x{:02X}) :: ", msg.group);

            match code {
                SassEventCode::GetCapabilityOfSass => {
                    println!("Get Capability (0x{:02X})", msg.code);
                }
                SassEventCode::NotifyCapabilityOfSass => {
                    println!("Notify Capability (0x{:02X})", msg.code);
                }
                SassEventCode::SetMultiPointState => {
                    println!("Set Multi-Point State (0x{:02X})", msg.code);
                }
                SassEventCode::SwitchAudioSourceBetweenConnectedDevices => {
                    println!("Switch Audio Source Between Connected Devices (0x{:02X})", msg.code);
                }
                SassEventCode::SwitchBack => {
                    println!("Switch Back (0x{:02X})", msg.code);
                }
                SassEventCode::NotifyMultiPointSwitchEvent => {
                    println!("Notify Multi-Point (0x{:02X})", msg.code);
                }
                SassEventCode::GetConnectionStatus => {
                    println!("Get Connection Status (0x{:02X})", msg.code);
                }
                SassEventCode::NotifyConnectionStatus => {
                    println!("Notify Connection Status (0x{:02X})", msg.code);
                }
                SassEventCode::SassInitiatedConnection => {
                    println!("SASS Initiated Connection (0x{:02X})", msg.code);
                }
                SassEventCode::IndicateInUseAccountKey => {
                    println!("Indicate In-Use Account Key (0x{:02X})", msg.code);
                }
                SassEventCode::SetCustomData => {
                    println!("Set Custom Data (0x{:02X})", msg.code);
                }
                SassEventCode::Unknown(_) | _ => {
                    println!("Unknown (0x{:02X})", msg.code);
                }
            }

            print_message_body_unknown(msg);
            println!();
        }
        EventGroup::Acknowledgement => {
            let code = AcknowledgementEventCode::from_primitive(msg.code);

            print!("Acknowledgement (0x{:02X}) ::", msg.group);

            match code {
                AcknowledgementEventCode::Ack => {
                    println!("ACK (0x{:02X})", msg.code);
                    println!("  group: 0x{:02X}", msg.data[0]);
                    println!("  code: 0x{:02X}", msg.data[1]);
                    println!();
                }
                AcknowledgementEventCode::Nak => {
                    println!("NAK (0x{:02X})", msg.code);
                    match msg.data[0] {
                        0x00 => println!("  reason: Not supported (0x00)"),
                        0x01 => println!("  reason: Device busy (0x01)"),
                        0x02 => println!("  reason: Not allowed due to current state (0x02)"),
                        _ => println!("  reason: Unknown (0x{:02X})", msg.data[0]),
                    }
                    println!("  group: 0x{:02X}", msg.data[1]);
                    println!("  code: 0x{:02X}", msg.data[2]);
                    println!();
                }
                AcknowledgementEventCode::Unknown(_) | _ => {
                    println!("Unknown (0x{:02X})", msg.code);
                    print_message_body_unknown(msg);
                    println!();
                }
            }
        }
        EventGroup::Unknown(_) | _ => {
            println!(
                "Unknown (0x{:02X}) :: Unknown (0x{:02X})",
                msg.group, msg.code
            );
            print_message_body_unknown(msg);
            println!();
        }
    }
}

fn print_message_body_unknown(msg: &Message) {
    let data = pretty_hex::config_hex(
        &msg.data,
        pretty_hex::HexConfig {
            title: false,
            ..Default::default()
        },
    );

    for line in data.lines() {
        println!("  {}", line);
    }
}
