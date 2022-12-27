mod bt;

use anyhow::Result;
use bluer::Address;
use clap::{Parser, Subcommand, ValueEnum, CommandFactory};
use futures::{Future, StreamExt};

use maestro::protocol::{utils, addr};
use maestro::pwrpc::client::{Client, ClientHandle};
use maestro::protocol::codec::Codec;
use maestro::service::MaestroService;
use maestro::service::settings::{self, Setting, SettingValue};


/// Control Google Pixel Buds Pro from the command line
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Device to use (search for compatible device if unspecified)
    #[arg(short, long, global=true)]
    device: Option<Address>,

    #[command(subcommand)]
    command: Command
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show device information
    Show {
        #[command(subcommand)]
        command: ShowCommand
    },

    /// Read settings value
    Get {
        #[command(subcommand)]
        setting: GetSetting
    },

    /// Write settings value
    Set {
        #[command(subcommand)]
        setting: SetSetting
    },
}

#[derive(Debug, Subcommand)]
enum ShowCommand {
    /// Show software information.
    Software,

    /// Show hardware information.
    Hardware,

    /// Show runtime information.
    Runtime,

    /// Show battery status.
    Battery,
}

#[derive(Debug, Subcommand)]
enum GetSetting {
    /// Get gesture state (enabled/disabled)
    Gestures,

    /// Get multipoint audio state (enabled/disabled)
    Multipoint,

    /// Get adaptive noise-cancelling gesture loop
    AncGestureLoop,

    /// Get adaptive noise-cancelling state
    Anc,

    /// Get volume-dependent EQ state (enabled/disabled)
    VolumeEq,

    /// Get 5-band EQ
    Eq,

    /// Get volume balance
    Balance,
}

#[derive(Debug, Subcommand)]
enum SetSetting {
    /// Enable/disable gestures
    Gestures {
        /// Whether to enable or disable gestures
        #[arg(action=clap::ArgAction::Set)]
        value: bool,
    },

    /// Enable/disable multipoint audio
    Multipoint {
        /// Whether to enable or disable multipoint audio
        #[arg(action=clap::ArgAction::Set)]
        value: bool,
    },

    /// Set adaptive noise-cancelling gesture loop
    AncGestureLoop {
        /// Enable 'off' mode in loop
        #[arg(action=clap::ArgAction::Set)]
        off: bool,

        /// Enable 'active' mode in loop
        #[arg(action=clap::ArgAction::Set)]
        active: bool,

        /// Enable 'aware' mode in loop
        #[arg(action=clap::ArgAction::Set)]
        aware: bool,
    },

    /// Set adaptive noise-cancelling state
    Anc {
        /// ANC state
        #[arg(value_enum)]
        value: AncState,
    },

    /// Enable/disable volume-dependent EQ
    VolumeEq {
        /// Whether to enable or disable volume-dependent EQ
        #[arg(action=clap::ArgAction::Set)]
        value: bool,
    },

    /// Set 5-band EQ
    Eq {
        /// Low-bass band (min: -6.0, max: 6.0)
        #[arg(value_parser=parse_eq_value)]
        low_bass: f32,

        /// Bass band (min: -6.0, max: 6.0)
        #[arg(value_parser=parse_eq_value)]
        bass: f32,

        /// Mid band (min: -6.0, max: 6.0)
        #[arg(value_parser=parse_eq_value)]
        mid: f32,

        /// Treble band (min: -6.0, max: 6.0)
        #[arg(value_parser=parse_eq_value)]
        treble: f32,

        /// Upper treble band (min: -6.0, max: 6.0)
        #[arg(value_parser=parse_eq_value)]
        upper_treble: f32,
    },

    /// Set volume balance
    Balance {
        /// Volume balance (-100 to +100)
        #[arg(value_parser=parse_balance)]
        value: i32,
    },
}

#[derive(Debug, ValueEnum, Clone, Copy)]
enum AncState {
    Off,
    Active,
    Aware,
}

impl From<AncState> for settings::AncState {
    fn from(value: AncState) -> Self {
        match value {
            AncState::Off => settings::AncState::Off,
            AncState::Active => settings::AncState::Active,
            AncState::Aware => settings::AncState::Aware,
        }
    }
}


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
            GetSetting::Gestures => {
                run(client, cmd_get_setting(handle, channel, settings::id::GestureEnable)).await
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
        },
        Command::Set { setting } => match setting {
            SetSetting::Gestures { value } => {
                let value = SettingValue::GestureEnable(value);
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
                let value = SettingValue::CurrentAncrState(value.into());
                run(client, cmd_set_setting(handle, channel, value)).await
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
            }
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
    println!("  case:      {} ({})", fw_ver_case,  fw_unk_case);
    println!("  left bud:  {} ({})", fw_ver_left,  fw_unk_left);
    println!("  right bud: {} ({})", fw_ver_right, fw_unk_right);

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
    println!("  case:      {}", serial_case);
    println!("  left bud:  {}", serial_left);
    println!("  right bud: {}", serial_right);

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
        println!("  case:      {}% ({})", lvl, bat_state_case);
    } else {
        println!("  case:      unknown");
    }
    if let Some(lvl) = bat_level_left {
        println!("  left bud:  {}% ({})", lvl, bat_state_left);
    } else {
        println!("  left bud:  unknown");
    }
    if let Some(lvl) = bat_level_right {
        println!("  right bud: {}% ({})", lvl, bat_state_right);
    } else {
        println!("  right bud: unknown");
    }
    println!();

    println!("placement:");
    println!("  left bud:  {}", place_left);
    println!("  right bud: {}", place_right);

    let address = addr::address_for_channel(channel);
    let peer_local = address.map(|a| a.source());
    let peer_remote = address.map(|a| a.target());

    println!();
    println!("connection:");
    if let Some(peer) = peer_local {
        println!("  local:  {:?}", peer);
    } else {
        println!("  local:  unknown");
    }
    if let Some(peer) = peer_remote {
        println!("  remote: {:?}", peer);
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
        println!("case:      {}% ({})", lvl, bat_state_case);
    } else {
        println!("case:      unknown");
    }
    if let Some(lvl) = bat_level_left {
        println!("left bud:  {}% ({})", lvl, bat_state_left);
    } else {
        println!("left bud:  unknown");
    }
    if let Some(lvl) = bat_level_right {
        println!("right bud: {}% ({})", lvl, bat_state_right);
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
    println!("{}", value);

    Ok(())
}

async fn cmd_set_setting(handle: ClientHandle, channel: u32, setting: SettingValue) -> Result<()> {
    let mut service = MaestroService::new(handle, channel);

    service.write_setting(setting).await?;
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

fn parse_eq_value(s: &str) -> std::result::Result<f32, String> {
    let val = s.parse().map_err(|e| format!("{}", e))?;

    if val > settings::EqBands::MAX_VALUE {
        Err(format!("exceeds maximum of {}", settings::EqBands::MAX_VALUE))
    } else if val < settings::EqBands::MIN_VALUE {
        Err(format!("exceeds minimum of {}", settings::EqBands::MIN_VALUE))
    } else {
        Ok(val)
    }
}

fn parse_balance(s: &str) -> std::result::Result<i32, String> {
    let val = s.parse().map_err(|e| format!("{}", e))?;

    if val > 100 {
        Err("exceeds maximum of 100".to_string())
    } else if val < -100 {
        Err("exceeds minimum of -100".to_string())
    } else {
        Ok(val)
    }
}
