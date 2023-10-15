//! Simple example for reading settings on the Pixel Buds Pro via the Maestro service.
//!
//! Usage:
//!   cargo run --example maestro_read_settings -- <bluetooth-device-address>

mod common;

use std::str::FromStr;

use anyhow::bail;
use bluer::{Address, Session};

use maestro::protocol::codec::Codec;
use maestro::protocol::utils;
use maestro::pwrpc::client::{Client, ClientHandle};
use maestro::service::MaestroService;
use maestro::service::settings::{self, SettingId};


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
    let settings_task = read_settings(handle, channel);

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

async fn read_settings(handle: ClientHandle, channel: u32) -> anyhow::Result<()> {
    let mut service = MaestroService::new(handle.clone(), channel);

    println!();
    println!("Read via types:");

    // read some typed settings via proxy structs
    let value = service.read_setting(settings::id::AutoOtaEnable).await?;
    println!("  Auto-OTA enabled:                    {}", value);

    let value = service.read_setting(settings::id::OhdEnable).await?;
    println!("  OHD enabled:                         {}", value);

    let value = service.read_setting(settings::id::OobeIsFinished).await?;
    println!("  OOBE finished:                       {}", value);

    let value = service.read_setting(settings::id::GestureEnable).await?;
    println!("  Gestures enabled:                    {}", value);

    let value = service.read_setting(settings::id::DiagnosticsEnable).await?;
    println!("  Diagnostics enabled:                 {}", value);

    let value = service.read_setting(settings::id::OobeMode).await?;
    println!("  OOBE mode:                           {}", value);

    let value = service.read_setting(settings::id::GestureControl).await?;
    println!("  Gesture control:                     {}", value);

    let value = service.read_setting(settings::id::MultipointEnable).await?;
    println!("  Multi-point enabled:                 {}", value);

    let value = service.read_setting(settings::id::AncrGestureLoop).await?;
    println!("  ANCR gesture loop:                   {}", value);

    let value = service.read_setting(settings::id::CurrentAncrState).await?;
    println!("  ANC status:                          {}", value);

    let value = service.read_setting(settings::id::OttsMode).await?;
    println!("  OTTS mode:                           {}", value);

    let value = service.read_setting(settings::id::VolumeEqEnable).await?;
    println!("  Volume-EQ enabled:                   {}", value);

    let value = service.read_setting(settings::id::CurrentUserEq).await?;
    println!("  Current user EQ:                     {}", value);

    let value = service.read_setting(settings::id::VolumeAsymmetry).await?;
    println!("  Volume balance/asymmetry:            {}", value);

    let value = service.read_setting(settings::id::SumToMono).await?;
    println!("  Mono output:                         {}", value);

    let value = service.read_setting(settings::id::VolumeExposureNotifications).await?;
    println!("  Volume level exposure notifications: {}", value);

    let value = service.read_setting(settings::id::SpeechDetection).await?;
    println!("  Speech detection:                    {}", value);

    // read settings via variant
    println!();
    println!("Read via variants:");

    let value = service.read_setting(SettingId::GestureEnable).await?;
    println!("  Gesture enable: {:?}", value);

    Ok(())
}
