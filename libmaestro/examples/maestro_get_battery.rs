//! Simple example for reading battery info via the Maestro service.
//!
//! Usage:
//!   cargo run --example maestro_get_battery -- <bluetooth-device-address>

mod common;

use std::str::FromStr;

use anyhow::bail;
use bluer::{Address, Session};
use futures::StreamExt;

use maestro::protocol::codec::Codec;
use maestro::protocol::types::RuntimeInfo;
use maestro::protocol::utils;
use maestro::pwrpc::client::{Client, ClientHandle};
use maestro::service::MaestroService;


#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();

    // handle command line arguments
    let addr = std::env::args().nth(1).expect("need device address as argument");
    let addr = Address::from_str(&addr)?;

    // set up session
    let session = Session::new().await?;
    let adapter = session.default_adapter().await?;

    println!("Using adapter '{}'", adapter.name());

    // get device
    let dev = adapter.device(addr)?;
    let uuids = {
        let mut uuids = Vec::from_iter(dev.uuids().await?
            .unwrap_or_default()
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

    println!("Connecting to Maestro profile");
    let stream = common::connect_maestro_rfcomm(&session, &dev).await?;

    println!("Profile connected");

    // set up stream for RPC communication
    let codec = Codec::new();
    let stream = codec.wrap(stream);

    // set up RPC client
    let mut client = Client::new(stream);
    let handle = client.handle();

    // retreive the channel numer
    let channel = utils::resolve_channel(&mut client).await?;

    let exec_task = common::run_client(client);
    let battery_task = get_battery(handle, channel);

    let info = tokio::select! {
        res = exec_task => {
            match res {
                Ok(_) => bail!("client terminated unexpectedly without error"),
                Err(e) => Err(e),
            }
        },
        res = battery_task => res,
    }?;

    let info = info.battery_info
        .expect("did not receive battery status in runtime-info-changed event");

    println!("Battery status:");

    if let Some(info) = info.case {
        match info.state {
            1 => println!("  case:  {}% (not charging)", info.level),
            2 => println!("  case:  {}% (charging)", info.level),
            x => println!("  case:  {}% (unknown state: {})", info.level, x),
        }
    } else {
        println!("  case: unknown");
    }

    if let Some(info) = info.left {
        match info.state {
            1 => println!("  left:  {}% (not charging)", info.level),
            2 => println!("  left:  {}% (charging)", info.level),
            x => println!("  left:  {}% (unknown state: {})", info.level, x),
        }
    } else {
        println!("  left: unknown");
    }

    if let Some(info) = info.right {
        match info.state {
            1 => println!("  right: {}% (not charging)", info.level),
            2 => println!("  right: {}% (charging)", info.level),
            x => println!("  right: {}% (unknown state: {})", info.level, x),
        }
    } else {
        println!("  right: unknown");
    }

    Ok(())
}

async fn get_battery(handle: ClientHandle, channel: u32) -> anyhow::Result<RuntimeInfo> {
    println!("Reading battery info...");
    println!();

    let mut service = MaestroService::new(handle, channel);

    let mut call = service.subscribe_to_runtime_info()?;
    let rt_info = if let Some(msg) = call.stream().next().await {
        msg?
    } else {
        bail!("did not receive any runtime-info event");
    };

    call.cancel_and_wait().await?;
    Ok(rt_info)
}
