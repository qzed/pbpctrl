use std::time::Duration;
use tokio::sync::mpsc;
use anyhow::Result;
use futures::StreamExt;
use maestro::protocol::codec::Codec;
use maestro::protocol::utils;
use maestro::protocol::addr;
use maestro::pwrpc::client::Client;
use maestro::service::MaestroService;
use maestro::service::settings::{self, SettingValue, Setting};
use maestro::protocol::types::RuntimeInfo as MRuntimeInfo;

use crate::bt;

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connected,
}

#[derive(Debug, Clone, Default)]
pub struct BatteryState {
    pub case_level: Option<u8>,
    pub case_status: String,
    pub left_level: Option<u8>,
    pub left_status: String,
    pub right_level: Option<u8>,
    pub right_status: String,
}

#[derive(Debug, Clone, Default)]
pub struct SoftwareInfo {
    pub case_version: String,
    pub left_version: String,
    pub right_version: String,
}

#[derive(Debug, Clone, Default)]
pub struct HardwareInfo {
    pub case_serial: String,
    pub left_serial: String,
    pub right_serial: String,
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeInfo {
    pub battery: BatteryState,
    pub placement_left: String,
    pub placement_right: String,
    pub peer_local: String,
    pub peer_remote: String,
}

#[derive(Debug, Clone)]
pub enum ClientEvent {
    ConnectionState(ConnectionState),
    Software(SoftwareInfo),
    Hardware(HardwareInfo),
    Runtime(RuntimeInfo),
    Setting(String, String), // key, value
    Error(String), 
}

#[derive(Debug, Clone)]
pub enum ClientCommand {
    CheckConnection,
    GetSoftware,
    GetHardware,
    GetSetting(String),
    SetSetting(String, String),
}

pub async fn run_loop(
    tx: mpsc::UnboundedSender<ClientEvent>,
    mut rx: mpsc::UnboundedReceiver<ClientCommand>,
) {
    let session = match bluer::Session::new().await {
        Ok(s) => s,
        Err(e) => {
            let _ = tx.send(ClientEvent::Error(format!("Bluetooth session error: {}", e)));
            return;
        }
    };

    let adapter = match session.default_adapter().await {
        Ok(a) => a,
        Err(e) => {
            let _ = tx.send(ClientEvent::Error(format!("Bluetooth adapter error: {}", e)));
            return;
        }
    };

    let _ = adapter.set_powered(true).await;

    loop {
        // 1. Establish connection
        let dev = match bt::find_maestro_device(&adapter).await {
            Ok(d) => d,
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(2)).await;
                if let Ok(_cmd) = rx.try_recv() {
                    // process minimal commands?
                }
                continue; 
            }
        };

        let stream = match bt::connect_maestro_rfcomm(&session, &dev).await {
            Ok(s) => s,
            Err(e) => {
                let _ = tx.send(ClientEvent::Error(format!("Connection failed: {}", e)));
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };

        let codec = Codec::new();
        let stream = codec.wrap(stream);
        let mut client = Client::new(stream);
        let handle = client.handle();

        let channel_res = tokio::time::timeout(
            Duration::from_secs(10),
            utils::resolve_channel(&mut client)
        ).await;

        let channel = match channel_res {
            Ok(Ok(c)) => c,
            Ok(Err(e)) => {
                 let _ = tx.send(ClientEvent::Error(format!("Channel resolution failed: {}", e)));
                 continue;
            }
            Err(_) => {
                 let _ = tx.send(ClientEvent::Error("Channel resolution timed out".to_string()));
                 continue;
            }
        };

        let mut service = MaestroService::new(handle.clone(), channel);
        let _ = tx.send(ClientEvent::ConnectionState(ConnectionState::Connected));

        // Subscribe to changes.
        let mut settings_sub = match service.subscribe_to_settings_changes() {
            Ok(call) => Some(call),
            Err(e) => {
                let _ = tx.send(ClientEvent::Error(format!("Settings sub failed: {}", e)));
                None
            }
        };
        
        let mut runtime_sub = match service.subscribe_to_runtime_info() {
            Ok(call) => Some(call),
            Err(e) => {
                 let _ = tx.send(ClientEvent::Error(format!("Runtime sub failed: {}", e)));
                 None
            }
        };

        // Spawn client run loop to ensure packet processing happens concurrently with command handling
        let mut client_task = tokio::spawn(async move { client.run().await });

        // Inner loop: Connected state
        loop {
            tokio::select! {
                res = &mut client_task => {
                    // Client task finished (error or disconnect)
                    let _ = tx.send(ClientEvent::ConnectionState(ConnectionState::Disconnected));
                    match res {
                        Ok(Err(e)) => { let _ = tx.send(ClientEvent::Error(format!("Client error: {}", e))); }
                        Ok(Ok(_)) => {} // Clean exit?
                        Err(e) => { let _ = tx.send(ClientEvent::Error(format!("Client task join error: {}", e))); }
                    }
                    break; 
                }
                
                cmd = rx.recv() => {
                    match cmd {
                        Some(c) => handle_command(c, &mut service, &tx).await,
                        None => return, 
                    }
                }
                
                Some(res) = async { settings_sub.as_mut()?.stream().next().await }, if settings_sub.is_some() => {
                    match res {
                        Ok(rsp) => {
                             if let Some(val) = rsp.value_oneof {
                                 use maestro::protocol::types::settings_rsp;
                                 let settings_rsp::ValueOneof::Value(sv) = val;
                                 // sv is types::SettingValue
                                 if let Some(vo) = sv.value_oneof {
                                     let setting: SettingValue = vo.into();
                                     process_setting_change(setting, &tx);
                                 }
                             }
                        }
                        Err(_) => break, 
                    }
                }
                
                Some(res) = async { runtime_sub.as_mut()?.stream().next().await }, if runtime_sub.is_some() => {
                    match res {
                        Ok(info) => {
                            let r_info = convert_runtime_info(info, channel);
                            let _ = tx.send(ClientEvent::Runtime(r_info));
                        }
                        Err(_) => break,
                    }
                }
            }
        }
        
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

async fn handle_command(cmd: ClientCommand, service: &mut MaestroService, tx: &mpsc::UnboundedSender<ClientEvent>) {
    match cmd {
        ClientCommand::CheckConnection => {
            let _ = tx.send(ClientEvent::ConnectionState(ConnectionState::Connected));
        }
        ClientCommand::GetSoftware => {
            match service.get_software_info().await {
                Ok(info) => {
                    let sw = SoftwareInfo {
                        case_version: info.firmware.as_ref().and_then(|f| f.case.as_ref()).map(|v| v.version_string.clone()).unwrap_or_default(),
                        left_version: info.firmware.as_ref().and_then(|f| f.left.as_ref()).map(|v| v.version_string.clone()).unwrap_or_default(),
                        right_version: info.firmware.as_ref().and_then(|f| f.right.as_ref()).map(|v| v.version_string.clone()).unwrap_or_default(),
                    };
                    let _ = tx.send(ClientEvent::Software(sw));
                }
                Err(e) => { let _ = tx.send(ClientEvent::Error(format!("GetSoftware failed: {}", e))); }
            }
        }
        ClientCommand::GetHardware => {
             match service.get_hardware_info().await {
                Ok(info) => {
                    let hw = HardwareInfo {
                        case_serial: info.serial_number.as_ref().map(|s| s.case.clone()).unwrap_or_default(),
                        left_serial: info.serial_number.as_ref().map(|s| s.left.clone()).unwrap_or_default(),
                        right_serial: info.serial_number.as_ref().map(|s| s.right.clone()).unwrap_or_default(),
                    };
                    let _ = tx.send(ClientEvent::Hardware(hw));
                }
                Err(e) => { let _ = tx.send(ClientEvent::Error(format!("GetHardware failed: {}", e))); }
            }
        }
        ClientCommand::GetSetting(key) => {
            let res = match key.as_str() {
                "anc" => read_and_send(service, settings::id::CurrentAncrState, &key, tx).await,
                "volume-eq" => read_and_send(service, settings::id::VolumeEqEnable, &key, tx).await,
                "mono" => read_and_send(service, settings::id::SumToMono, &key, tx).await,
                "speech-detection" => read_and_send(service, settings::id::SpeechDetection, &key, tx).await,
                "multipoint" => read_and_send(service, settings::id::MultipointEnable, &key, tx).await,
                "ohd" => read_and_send(service, settings::id::OhdEnable, &key, tx).await,
                "gestures" => read_and_send(service, settings::id::GestureEnable, &key, tx).await,
                "volume-exposure-notifications" => read_and_send(service, settings::id::VolumeExposureNotifications, &key, tx).await,
                "diagnostics" => read_and_send(service, settings::id::DiagnosticsEnable, &key, tx).await,
                "oobe-mode" => read_and_send(service, settings::id::OobeMode, &key, tx).await,
                "oobe-is-finished" => read_and_send(service, settings::id::OobeIsFinished, &key, tx).await,
                "balance" => read_and_send(service, settings::id::VolumeAsymmetry, &key, tx).await,
                "eq" => read_and_send(service, settings::id::CurrentUserEq, &key, tx).await,
                "gesture-control" => read_and_send(service, settings::id::GestureControl, &key, tx).await,
                _ => Ok(()),
            };
            if let Err(e) = res {
                let _ = tx.send(ClientEvent::Error(format!("Get {} failed: {}", key, e)));
            }
        }
        ClientCommand::SetSetting(key, val) => {
            let res = match key.as_str() {
                "anc" => {
                    let state = match val.as_str() {
                        "active" => settings::AncState::Active,
                        "aware" => settings::AncState::Aware,
                        "off" => settings::AncState::Off,
                        "adaptive" => settings::AncState::Adaptive, 
                        _ => settings::AncState::Off, 
                    };
                    service.write_setting(SettingValue::CurrentAncrState(state)).await
                },
                "volume-eq" => service.write_setting(SettingValue::VolumeEqEnable(val == "true")).await,
                "mono" => service.write_setting(SettingValue::SumToMono(val == "true")).await,
                "speech-detection" => service.write_setting(SettingValue::SpeechDetection(val == "true")).await,
                "multipoint" => service.write_setting(SettingValue::MultipointEnable(val == "true")).await,
                "ohd" => service.write_setting(SettingValue::OhdEnable(val == "true")).await,
                "gestures" => service.write_setting(SettingValue::GestureEnable(val == "true")).await,
                "volume-exposure-notifications" => service.write_setting(SettingValue::VolumeExposureNotifications(val == "true")).await,
                "diagnostics" => service.write_setting(SettingValue::DiagnosticsEnable(val == "true")).await,
                "oobe-mode" => service.write_setting(SettingValue::OobeMode(val == "true")).await,
                "oobe-is-finished" => service.write_setting(SettingValue::OobeIsFinished(val == "true")).await,
                "balance" => {
                    if let Ok(n) = val.parse::<i32>() {
                         let va = settings::VolumeAsymmetry::from_normalized(n);
                         service.write_setting(SettingValue::VolumeAsymmetry(va)).await
                    } else {
                        Ok(())
                    }
                },
                "eq" => {
                    let parts: Vec<f32> = val.split_whitespace().filter_map(|s| s.parse().ok()).collect();
                    if parts.len() == 5 {
                        let eq = settings::EqBands::new(parts[0], parts[1], parts[2], parts[3], parts[4]);
                        service.write_setting(SettingValue::CurrentUserEq(eq)).await
                    } else {
                        Ok(())
                    }
                },
                _ => Ok(()),
            };
            
             if let Err(e) = res {
                let _ = tx.send(ClientEvent::Error(format!("Set {} failed: {}", key, e)));
            } 
        }
    }
}

async fn read_and_send<T>(service: &mut MaestroService, setting: T, key: &str, tx: &mpsc::UnboundedSender<ClientEvent>) -> Result<(), maestro::pwrpc::Error>
where T: Setting, T::Type: std::fmt::Display {
    let val = service.read_setting(setting).await?;
    let _ = tx.send(ClientEvent::Setting(key.to_string(), val.to_string()));
    Ok(())
}

fn process_setting_change(setting: SettingValue, tx: &mpsc::UnboundedSender<ClientEvent>) {
    let (key, val) = match setting {
        SettingValue::CurrentAncrState(s) => ("anc", s.to_string()),
        SettingValue::VolumeEqEnable(b) => ("volume-eq", b.to_string()),
        SettingValue::SumToMono(b) => ("mono", b.to_string()),
        SettingValue::SpeechDetection(b) => ("speech-detection", b.to_string()),
        SettingValue::MultipointEnable(b) => ("multipoint", b.to_string()),
        SettingValue::OhdEnable(b) => ("ohd", b.to_string()),
        SettingValue::GestureEnable(b) => ("gestures", b.to_string()),
        SettingValue::VolumeExposureNotifications(b) => ("volume-exposure-notifications", b.to_string()),
        SettingValue::DiagnosticsEnable(b) => ("diagnostics", b.to_string()),
        SettingValue::OobeMode(b) => ("oobe-mode", b.to_string()),
        SettingValue::OobeIsFinished(b) => ("oobe-is-finished", b.to_string()),
        SettingValue::VolumeAsymmetry(va) => ("balance", va.to_string()),
        SettingValue::CurrentUserEq(eq) => ("eq", eq.to_string()),
        SettingValue::GestureControl(gc) => ("gesture-control", format!("{:?}", gc)), 
        _ => return,
    };
    
    let _ = tx.send(ClientEvent::Setting(key.to_string(), val.to_lowercase()));
}

fn convert_runtime_info(info: MRuntimeInfo, channel: u32) -> RuntimeInfo {
    let address = addr::address_for_channel(channel);
    let peer_local = address.map(|a| format!("{:?}", a.source())).unwrap_or_else(|| "unknown".to_string());
    let peer_remote = address.map(|a| format!("{:?}", a.target())).unwrap_or_else(|| "unknown".to_string());

    RuntimeInfo {
        battery: BatteryState {
            case_level: info.battery_info.as_ref().and_then(|b| b.case.as_ref()).map(|b| b.level as u8),
            case_status: info.battery_info.as_ref().and_then(|b| b.case.as_ref()).map(|b| if b.state == 2 { "charging" } else { "not charging" }).unwrap_or("unknown").to_string(),
            left_level: info.battery_info.as_ref().and_then(|b| b.left.as_ref()).map(|b| b.level as u8),
            left_status: info.battery_info.as_ref().and_then(|b| b.left.as_ref()).map(|b| if b.state == 2 { "charging" } else { "not charging" }).unwrap_or("unknown").to_string(),
            right_level: info.battery_info.as_ref().and_then(|b| b.right.as_ref()).map(|b| b.level as u8),
            right_status: info.battery_info.as_ref().and_then(|b| b.right.as_ref()).map(|b| if b.state == 2 { "charging" } else { "not charging" }).unwrap_or("unknown").to_string(),
        },
        placement_left: info.placement.as_ref().map(|p| if p.left_bud_in_case { "in case" } else { "out of case" }).unwrap_or("unknown").to_string(),
        placement_right: info.placement.as_ref().map(|p| if p.right_bud_in_case { "in case" } else { "out of case" }).unwrap_or("unknown").to_string(),
        peer_local,
        peer_remote,
    }
}
