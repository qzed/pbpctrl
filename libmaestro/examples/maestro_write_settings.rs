//! Simple example for changing settings on the Pixel Buds Pro via the Maestro service.
//!
//! Sets active nois ecancelling (ANC) state. 1: off, 2: active, 3: aware.
//!
//! Usage:
//!   cargo run --example maestro_write_settings -- <bluetooth-device-address> <anc-state>

mod common;

use std::str::FromStr;

use anyhow::bail;
use bluer::{Address, Session};
use maestro::protocol::utils;
use num_enum::FromPrimitive;

use maestro::protocol::codec::Codec;
use maestro::pwrpc::client::{Client, ClientHandle};
use maestro::service::MaestroService;
use maestro::service::settings::{AncState, SettingValue};


#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();

    // handle command line arguments
    let addr = std::env::args().nth(1).expect("need device address as argument");
    let addr = Address::from_str(&addr)?;

    let anc_state = std::env::args().nth(2).expect("need ANC state as argument");
    let anc_state = i32::from_str(&anc_state)?;
    let anc_state = AncState::from_primitive(anc_state);

    if let AncState::Unknown(x) = anc_state {
        bail!("invalid ANC state {x}");
    }

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
    let settings_task = read_settings(handle, channel, anc_state);

    tokio::select! {
        res = exec_task => {
            match res {
                Ok(_) => bail!("client terminated unexpectedly without error"),
                Err(e) => Err(e),
            }
        },
        res = settings_task => res,
    }
}

async fn read_settings(handle: ClientHandle, channel: u32, anc_state: AncState) -> anyhow::Result<()> {
    let mut service = MaestroService::new(handle.clone(), channel);

    println!();
    println!("Setting ANC status to '{}'", anc_state);

    service.write_setting(SettingValue::CurrentAncrState(anc_state)).await?;

    Ok(())
}
