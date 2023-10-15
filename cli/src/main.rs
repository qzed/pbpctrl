mod bt;
mod cli;

use anyhow::Result;
use clap::{Parser, CommandFactory};
use futures::{Future, StreamExt};

use maestro::protocol::{utils, addr};
use maestro::pwrpc::client::{Client, ClientHandle};
use maestro::protocol::codec::Codec;
use maestro::service::MaestroService;
use maestro::service::settings::{self, Setting, SettingValue};

use cli::*;


#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    // set up session
    let session = bluer::Session::new().await?;
    let adapter = session.default_adapter().await?;

    // set up device
    let dev = if let Some(address) = args.device {
        tracing::debug!("using provided address: {}", address);
        adapter.device(address)?
    } else {
        tracing::debug!("no device specified, searching for compatible one");
        bt::find_maestro_device(&adapter).await?
    };

    // set up profile
    let stream = bt::connect_maestro_rfcomm(&session, &dev).await?;

    // set up codec
    let codec = Codec::new();
    let stream = codec.wrap(stream);

    // set up RPC client
    let mut client = Client::new(stream);
    let handle = client.handle();

    // resolve channel
    let channel = utils::resolve_channel(&mut client).await?;

    match args.command {
        Command::Show { command } => match command {
            ShowCommand::Software => run(client, cmd_show_software(handle, channel)).await,
            ShowCommand::Hardware => run(client, cmd_show_hardware(handle, channel)).await,
            ShowCommand::Runtime => run(client, cmd_show_runtime(handle, channel)).await,
            ShowCommand::Battery => run(client, cmd_show_battery(handle, channel)).await,
        },
        Command::Get { setting } => match setting {
            GetSetting::AutoOta => {
                run(client, cmd_get_setting(handle, channel, settings::id::AutoOtaEnable)).await
            },
            GetSetting::Ohd => {
                run(client, cmd_get_setting(handle, channel, settings::id::OhdEnable)).await
            },
            GetSetting::OobeIsFinished => {
                run(client, cmd_get_setting(handle, channel, settings::id::OobeIsFinished)).await
            },
            GetSetting::Gestures => {
                run(client, cmd_get_setting(handle, channel, settings::id::GestureEnable)).await
            },
            GetSetting::Diagnostics => {
                run(client, cmd_get_setting(handle, channel, settings::id::DiagnosticsEnable)).await
            }
            GetSetting::OobeMode => {
                run(client, cmd_get_setting(handle, channel, settings::id::OobeMode)).await
            },
            GetSetting::GestureControl => {
                run(client, cmd_get_setting(handle, channel, settings::id::GestureControl)).await
            },
            GetSetting::Multipoint => {
                run(client, cmd_get_setting(handle, channel, settings::id::MultipointEnable)).await
            },
            GetSetting::AncGestureLoop => {
                run(client, cmd_get_setting(handle, channel, settings::id::AncrGestureLoop)).await
            }
            GetSetting::Anc => {
                run(client, cmd_get_setting(handle, channel, settings::id::CurrentAncrState)).await
            },
            GetSetting::VolumeEq => {
                run(client, cmd_get_setting(handle, channel, settings::id::VolumeEqEnable)).await
            },
            GetSetting::Eq => {
                run(client, cmd_get_setting(handle, channel, settings::id::CurrentUserEq)).await
            },
            GetSetting::Balance => {
                run(client, cmd_get_setting(handle, channel, settings::id::VolumeAsymmetry)).await
            },
            GetSetting::Mono => {
                run(client, cmd_get_setting(handle, channel, settings::id::SumToMono)).await
            },
            GetSetting::VolumeExposureNotifications => {
                run(client, cmd_get_setting(handle, channel, settings::id::VolumeExposureNotifications)).await
            },
            GetSetting::SpeechDetection => {
                run(client, cmd_get_setting(handle, channel, settings::id::SpeechDetection)).await
            },
        },
        Command::Set { setting } => match setting {
            SetSetting::AutoOta { value } => {
                let value = SettingValue::AutoOtaEnable(value);
                run(client, cmd_set_setting(handle, channel, value)).await
            },
            SetSetting::Ohd { value } => {
                let value = SettingValue::OhdEnable(value);
                run(client, cmd_set_setting(handle, channel, value)).await
            },
            SetSetting::OobeIsFinished { value } => {
                let value = SettingValue::OobeIsFinished(value);
                run(client, cmd_set_setting(handle, channel, value)).await
            },
            SetSetting::Gestures { value } => {
                let value = SettingValue::GestureEnable(value);
                run(client, cmd_set_setting(handle, channel, value)).await
            },
            SetSetting::Diagnostics { value } => {
                let value = SettingValue::DiagnosticsEnable(value);
                run(client, cmd_set_setting(handle, channel, value)).await
            },
            SetSetting::OobeMode { value } => {
                let value = SettingValue::OobeMode(value);
                run(client, cmd_set_setting(handle, channel, value)).await
            },
            SetSetting::GestureControl { left, right } => {
                let value = settings::GestureControl { left: left.into(), right: right.into() };
                let value = SettingValue::GestureControl(value);
                run(client, cmd_set_setting(handle, channel, value)).await
            },
            SetSetting::Multipoint { value } => {
                let value = SettingValue::MultipointEnable(value);
                run(client, cmd_set_setting(handle, channel, value)).await
            },
            SetSetting::AncGestureLoop { off, active, aware } => {
                let value = settings::AncrGestureLoop { off, active, aware };

                if !value.is_valid() {
                    use clap::error::ErrorKind;

                    let mut cmd = Args::command();
                    let err = cmd.error(
                        ErrorKind::InvalidValue,
                        "This command requires at least tow enabled ('true') modes"
                    );
                    err.exit();
                }

                let value = SettingValue::AncrGestureLoop(value);
                run(client, cmd_set_setting(handle, channel, value)).await
            },
            SetSetting::Anc { value } => {
                match value {
                    AncState::Off => {
                        let value = SettingValue::CurrentAncrState(settings::AncState::Off);
                        run(client, cmd_set_setting(handle, channel, value)).await
                    },
                    AncState::Aware => {
                        let value = SettingValue::CurrentAncrState(settings::AncState::Aware);
                        run(client, cmd_set_setting(handle, channel, value)).await
                    },
                    AncState::Active => {
                        let value = SettingValue::CurrentAncrState(settings::AncState::Active);
                        run(client, cmd_set_setting(handle, channel, value)).await
                    },
                    AncState::CycleNext => {
                        run(client, cmd_anc_cycle(handle, channel, true)).await
                    },
                    AncState::CyclePrev => {
                        run(client, cmd_anc_cycle(handle, channel, false)).await
                    },
                }
            },
            SetSetting::VolumeEq { value } => {
                let value = SettingValue::VolumeEqEnable(value);
                run(client, cmd_set_setting(handle, channel, value)).await
            },
            SetSetting::Eq { low_bass, bass, mid, treble, upper_treble } => {
                let value = settings::EqBands::new(low_bass, bass, mid, treble, upper_treble);
                let value = SettingValue::CurrentUserEq(value);
                run(client, cmd_set_setting(handle, channel, value)).await
            },
            SetSetting::Balance { value } => {
                let value = settings::VolumeAsymmetry::from_normalized(value);
                let value = SettingValue::VolumeAsymmetry(value);
                run(client, cmd_set_setting(handle, channel, value)).await
            },
            SetSetting::Mono { value } => {
                let value = SettingValue::SumToMono(value);
                run(client, cmd_set_setting(handle, channel, value)).await
            },
            SetSetting::VolumeExposureNotifications { value } => {
                let value = SettingValue::VolumeExposureNotifications(value);
                run(client, cmd_set_setting(handle, channel, value)).await
            },
            SetSetting::SpeechDetection { value } => {
                let value = SettingValue::SpeechDetection(value);
                run(client, cmd_set_setting(handle, channel, value)).await
            },
        },
    }
}

async fn cmd_show_software(handle: ClientHandle, channel: u32) -> Result<()> {
    let mut service = MaestroService::new(handle, channel);

    let info = service.get_software_info().await?;

    let fw_ver_case = info.firmware.as_ref()
        .and_then(|fw| fw.case.as_ref())
        .map(|fw| fw.version_string.as_str())
        .unwrap_or("unknown");

    let fw_ver_left = info.firmware.as_ref()
        .and_then(|fw| fw.left.as_ref())
        .map(|fw| fw.version_string.as_str())
        .unwrap_or("unknown");

    let fw_ver_right = info.firmware.as_ref()
        .and_then(|fw| fw.right.as_ref())
        .map(|fw| fw.version_string.as_str())
        .unwrap_or("unknown");

    let fw_unk_case = info.firmware.as_ref()
        .and_then(|fw| fw.case.as_ref())
        .map(|fw| fw.unknown.as_str())
        .unwrap_or("unknown");

    let fw_unk_left = info.firmware.as_ref()
        .and_then(|fw| fw.left.as_ref())
        .map(|fw| fw.unknown.as_str())
        .unwrap_or("unknown");

    let fw_unk_right = info.firmware.as_ref()
        .and_then(|fw| fw.right.as_ref())
        .map(|fw| fw.unknown.as_str())
        .unwrap_or("unknown");

    println!("firmware:");
    println!("  case:      {fw_ver_case} ({fw_unk_case})");
    println!("  left bud:  {fw_ver_left} ({fw_unk_left})");
    println!("  right bud: {fw_ver_right} ({fw_unk_right})");

    Ok(())
}

async fn cmd_show_hardware(handle: ClientHandle, channel: u32) -> Result<()> {
    let mut service = MaestroService::new(handle, channel);

    let info = service.get_hardware_info().await?;

    let serial_case = info.serial_number.as_ref()
        .map(|ser| ser.case.as_str())
        .unwrap_or("unknown");

    let serial_left = info.serial_number.as_ref()
        .map(|ser| ser.left.as_str())
        .unwrap_or("unknown");

    let serial_right = info.serial_number.as_ref()
        .map(|ser| ser.right.as_str())
        .unwrap_or("unknown");

    println!("serial numbers:");
    println!("  case:      {serial_case}");
    println!("  left bud:  {serial_left}");
    println!("  right bud: {serial_right}");

    Ok(())
}

async fn cmd_show_runtime(handle: ClientHandle, channel: u32) -> Result<()> {
    let mut service = MaestroService::new(handle, channel);

    let mut call = service.subscribe_to_runtime_info()?;

    let info = call.stream().next().await
        .ok_or_else(|| anyhow::anyhow!("stream terminated without item"))??;

    let bat_level_case = info.battery_info.as_ref()
        .and_then(|b| b.case.as_ref())
        .map(|b| b.level);

    let bat_state_case = info.battery_info.as_ref()
        .and_then(|b| b.case.as_ref())
        .map(|b| if b.state == 2 { "charging" } else if b.state == 1 { "not charging" } else { "unknown" })
        .unwrap_or("unknown");

    let bat_level_left = info.battery_info.as_ref()
        .and_then(|b| b.left.as_ref())
        .map(|b| b.level);

    let bat_state_left = info.battery_info.as_ref()
        .and_then(|b| b.left.as_ref())
        .map(|b| if b.state == 2 { "charging" } else if b.state == 1 { "not charging" } else { "unknown" })
        .unwrap_or("unknown");

    let bat_level_right = info.battery_info.as_ref()
        .and_then(|b| b.right.as_ref())
        .map(|b| b.level);

    let bat_state_right = info.battery_info.as_ref()
        .and_then(|b| b.right.as_ref())
        .map(|b| if b.state == 2 { "charging" } else if b.state == 1 { "not charging" } else { "unknown" })
        .unwrap_or("unknown");

    let place_left = info.placement.as_ref()
        .map(|p| if p.left_bud_in_case { "in case" } else { "out of case" })
        .unwrap_or("unknown");

    let place_right = info.placement.as_ref()
        .map(|p| if p.right_bud_in_case { "in case" } else { "out of case" })
        .unwrap_or("unknown");

    println!("clock: {} ms", info.timestamp_ms);
    println!();

    println!("battery:");
    if let Some(lvl) = bat_level_case {
        println!("  case:      {lvl}% ({bat_state_case})");
    } else {
        println!("  case:      unknown");
    }
    if let Some(lvl) = bat_level_left {
        println!("  left bud:  {lvl}% ({bat_state_left})");
    } else {
        println!("  left bud:  unknown");
    }
    if let Some(lvl) = bat_level_right {
        println!("  right bud: {lvl}% ({bat_state_right})");
    } else {
        println!("  right bud: unknown");
    }
    println!();

    println!("placement:");
    println!("  left bud:  {place_left}");
    println!("  right bud: {place_right}");

    let address = addr::address_for_channel(channel);
    let peer_local = address.map(|a| a.source());
    let peer_remote = address.map(|a| a.target());

    println!();
    println!("connection:");
    if let Some(peer) = peer_local {
        println!("  local:  {peer:?}");
    } else {
        println!("  local:  unknown");
    }
    if let Some(peer) = peer_remote {
        println!("  remote: {peer:?}");
    } else {
        println!("  remote: unknown");
    }

    Ok(())
}

async fn cmd_show_battery(handle: ClientHandle, channel: u32) -> Result<()> {
    let mut service = MaestroService::new(handle, channel);

    let mut call = service.subscribe_to_runtime_info()?;

    let info = call.stream().next().await
        .ok_or_else(|| anyhow::anyhow!("stream terminated without item"))??;

    let bat_level_case = info.battery_info.as_ref()
        .and_then(|b| b.case.as_ref())
        .map(|b| b.level);

    let bat_state_case = info.battery_info.as_ref()
        .and_then(|b| b.case.as_ref())
        .map(|b| if b.state == 2 { "charging" } else if b.state == 1 { "not charging" } else { "unknown" })
        .unwrap_or("unknown");

    let bat_level_left = info.battery_info.as_ref()
        .and_then(|b| b.left.as_ref())
        .map(|b| b.level);

    let bat_state_left = info.battery_info.as_ref()
        .and_then(|b| b.left.as_ref())
        .map(|b| if b.state == 2 { "charging" } else if b.state == 1 { "not charging" } else { "unknown" })
        .unwrap_or("unknown");

    let bat_level_right = info.battery_info.as_ref()
        .and_then(|b| b.right.as_ref())
        .map(|b| b.level);

    let bat_state_right = info.battery_info.as_ref()
        .and_then(|b| b.right.as_ref())
        .map(|b| if b.state == 2 { "charging" } else if b.state == 1 { "not charging" } else { "unknown" })
        .unwrap_or("unknown");

    if let Some(lvl) = bat_level_case {
        println!("case:      {lvl}% ({bat_state_case})");
    } else {
        println!("case:      unknown");
    }
    if let Some(lvl) = bat_level_left {
        println!("left bud:  {lvl}% ({bat_state_left})");
    } else {
        println!("left bud:  unknown");
    }
    if let Some(lvl) = bat_level_right {
        println!("right bud: {lvl}% ({bat_state_right})");
    } else {
        println!("right bud: unknown");
    }

    Ok(())
}

async fn cmd_get_setting<T>(handle: ClientHandle, channel: u32, setting: T) -> Result<()>
where
    T: Setting,
    T::Type: std::fmt::Display,
{
    let mut service = MaestroService::new(handle, channel);

    let value = service.read_setting(setting).await?;
    println!("{value}");

    Ok(())
}

async fn cmd_set_setting(handle: ClientHandle, channel: u32, setting: SettingValue) -> Result<()> {
    let mut service = MaestroService::new(handle, channel);

    service.write_setting(setting).await?;
    Ok(())
}

async fn cmd_anc_cycle(handle: ClientHandle, channel: u32, forward: bool) -> Result<()> {
    let mut service = MaestroService::new(handle, channel);

    let enabled = service.read_setting(settings::id::AncrGestureLoop).await?;
    let state = service.read_setting(settings::id::CurrentAncrState).await?;

    if let settings::AncState::Unknown(x) = state {
        anyhow::bail!("unknown ANC state: {x}");
    }

    let states = [
        (settings::AncState::Active, enabled.active),
        (settings::AncState::Off, enabled.off),
        (settings::AncState::Aware, enabled.aware),
    ];

    let index = states.iter().position(|(s, _)| *s == state).unwrap();

    for offs in 1..states.len() {
        let next = if forward {
            index + offs
        } else {
            index + states.len() - offs
        } % states.len();

        let (state, enabled) = states[next];
        if enabled {
            service.write_setting(SettingValue::CurrentAncrState(state)).await?;
            break;
        }
    }

    Ok(())
}

pub async fn run<S, E, F>(mut client: Client<S>, task: F) -> Result<()>
where
    S: futures::Sink<maestro::pwrpc::types::RpcPacket>,
    S: futures::Stream<Item = Result<maestro::pwrpc::types::RpcPacket, E>> + Unpin,
    maestro::pwrpc::Error: From<E>,
    maestro::pwrpc::Error: From<S::Error>,
    F: Future<Output=Result<(), anyhow::Error>>,
{
    tokio::select! {
        res = client.run() => {
            res?;
            anyhow::bail!("client terminated unexpectedly");
        },
        res = task => {
            res?;
            tracing::trace!("task terminated successfully");
        }
        sig = tokio::signal::ctrl_c() => {
            sig?;
            tracing::trace!("client termination requested");
        },
    }

    client.terminate().await?;

    tracing::trace!("client terminated successfully");
    Ok(())
}
