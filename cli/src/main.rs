mod bt;

use anyhow::Result;
use bluer::Address;
use clap::{Parser, Subcommand};

use futures::{Future, StreamExt};
use maestro::protocol::{utils, addr};
use maestro::pwrpc::client::{Client, ClientHandle};
use maestro::protocol::codec::Codec;
use maestro::service::MaestroService;


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