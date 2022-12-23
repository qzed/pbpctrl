//! Simple example for reading settings on the Pixel Buds Pro via the Maestro service.
//!
//! Usage:
//!   cargo run --example maestro_read_settings -- <bluetooth-device-address>

use std::str::FromStr;

use anyhow::bail;

use bluer::{Address, Session, Device};
use bluer::rfcomm::{Profile, ReqError, Role, ProfileHandle};

use futures::{StreamExt, Sink};

use maestro::protocol::codec::Codec;
use maestro::pwrpc::client::{Client, ClientHandle};
use maestro::pwrpc::types::RpcPacket;
use maestro::pwrpc::Error;
use maestro::service::MaestroService;
use maestro::service::settings::{self, SettingId};


#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), anyhow::Error> {
    env_logger::init();

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

    let stream = {
        // register GFPS profile
        println!("Registering Maestro profile...");

        let profile = Profile {
            uuid: maestro::UUID,
            role: Some(Role::Client),
            require_authentication: Some(false),
            require_authorization: Some(false),
            auto_connect: Some(false),
            ..Default::default()
        };

        let mut profile_handle = session.register_profile(profile).await?;

        // connect profile
        println!("Connecting GFPS profile...");
        connect_device_to_profile(&mut profile_handle, &dev).await?
    };

    println!("Profile connected");

    // set up stream for RPC communication
    let codec = Codec::new();
    let mut stream = codec.wrap(stream);

    // retreive the channel numer
    //
    // Note: this is a bit hacky. The protocol works with different channels,
    // depending on which bud is active (or case...), and which peer we
    // represent (Maestro A or B). Only one is responsive and ther doesn't seem
    // to be a good way to figure out which.
    //
    // The app seems to do this by firing off one GetSoftwareInfo request per
    // potential channel, waiting for responses and choosing the responsive
    // one. However, the buds also automatically send one GetSoftwareInfo
    // response on the right channel without a request right after establishing
    // a connection. So for now we just listen for that first message,
    // discarding all but the channel id.

    let mut channel = 0;

    while let Some(packet) = stream.next().await {
        match packet {
            Ok(packet) => {
                channel = packet.channel_id;
                break;
            }
            Err(e) => {
                Err(e)?
            }
        }
    }

    // set up RPC client
    let client = Client::new(stream);
    let handle = client.handle();

    let exec_task = run_client(client);
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

async fn read_settings<S, E>(handle: ClientHandle<S>, channel: u32) -> anyhow::Result<()>
where
    S: Sink<RpcPacket>,
    S: futures::Stream<Item = Result<RpcPacket, E>> + Unpin,
    Error: From<E>,
    Error: From<S::Error>,
{
    let service = MaestroService::new(handle.clone(), channel);

    println!();
    println!("Read via types:");

    // read some typed settings via proxy structs
    let value = service.read_setting(settings::id::AutoOtaEnable).await?;
    println!("  Auto-OTA enabled:         {}", value);

    let value = service.read_setting(settings::id::OhdEnable).await?;
    println!("  OHD enabled:              {}", value);

    let value = service.read_setting(settings::id::OobeIsFinished).await?;
    println!("  OOBE finished:            {}", value);

    let value = service.read_setting(settings::id::GestureEnable).await?;
    println!("  Gestures enabled:         {}", value);

    let value = service.read_setting(settings::id::DiagnosticsEnable).await?;
    println!("  Diagnostics enabled:      {}", value);

    let value = service.read_setting(settings::id::OobeMode).await?;
    println!("  OOBE mode:                {}", value);

    let value = service.read_setting(settings::id::GestureControl).await?;
    println!("  Gesture control:          {}", value);

    let value = service.read_setting(settings::id::MultipointEnable).await?;
    println!("  Multi-point enabled:      {}", value);

    let value = service.read_setting(settings::id::AncrGestureLoop).await?;
    println!("  ANCR gesture loop:        {}", value);

    let value = service.read_setting(settings::id::CurrentAncrState).await?;
    println!("  ANC status:               {}", value);

    let value = service.read_setting(settings::id::OttsMode).await?;
    println!("  OTTS mode:                {}", value);

    let value = service.read_setting(settings::id::VolumeEqEnable).await?;
    println!("  Volume-EQ enabled:        {}", value);

    let value = service.read_setting(settings::id::CurrentUserEq).await?;
    println!("  Current user EQ:          {}", value);

    let value = service.read_setting(settings::id::VolumeAsymmetry).await?;
    println!("  Volume balance/asymmetry: {}", value);

    // read settings via variant
    println!();
    println!("Read via variants:");

    let value = service.read_setting(SettingId::GestureEnable).await?;
    println!("  Gesture enable: {:?}", value);

    Ok(())
}

async fn run_client<S, E>(mut client: Client<S>) -> anyhow::Result<()>
where
    S: Sink<RpcPacket>,
    S: futures::Stream<Item = Result<RpcPacket, E>> + Unpin,
    Error: From<E>,
    Error: From<S::Error>,
{
    client.run().await?;
    Ok(())
}

async fn connect_device_to_profile(profile: &mut ProfileHandle, dev: &Device)
    -> bluer::Result<bluer::rfcomm::Stream>
{
    loop {
        tokio::select! {
            res = async {
                let _ = dev.connect().await;
                dev.connect_profile(&maestro::UUID).await
            } => {
                if let Err(err) = res {
                    println!("Connecting GFPS profile failed: {:?}", err);
                }
                tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
            },
            req = profile.next() => {
                let req = req.expect("no connection request received");

                if req.device() == dev.address() {
                    println!("Accepting request...");
                    break req.accept();
                } else {
                    println!("Rejecting unknown device {}", req.device());
                    req.reject(ReqError::Rejected);
                }
            },
        }
    }
}
