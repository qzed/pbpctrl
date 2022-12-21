//! Simple example for listening to Maestro messages sent via the RFCOMM channel.
//!
//! Usage:
//!   cargo run --example maestro_listen -- <bluetooth-device-address>

use std::str::FromStr;

use bluer::{Address, Session, Device};
use bluer::rfcomm::{Profile, ReqError, Role, ProfileHandle};

use futures::{StreamExt, Sink};

use maestro::pwrpc::client::{Client, Request, Streaming};
use maestro::pwrpc::codec::{Codec, Packet};
use maestro::pwrpc::id::Identifier;
use maestro::protocol::addr;
use maestro::protocol::types::{SoftwareInfo, SettingsRsp};


#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug")
    );

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

    // try to reconnect if connection is reset
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

    // listen to event messages
    let codec = Codec::new();
    let mut stream = codec.wrap(stream);

    println!("Listening...");
    println!();

    let mut packet_addr = addr::Address::from_value(0);

    while let Some(packet) = stream.next().await {
        match packet {
            Ok(packet) => {
                packet_addr = addr::Address::from_value(packet.address).swap();
                break;
            }
            Err(e) => {
                Err(e)?
            }
        }
    }

    let client = Client::new(stream);
    let handle = client.handle();

    tokio::spawn(run_client(client));

    let req = Request {
        address: packet_addr,
        channel_id: packet_addr.channel_id().unwrap(),
        service_id: Identifier::new("maestro_pw.Maestro").hash(),
        method_id: Identifier::new("GetSoftwareInfo").hash(),
        call_id: 42,
        message: maestro::protocol::types::Empty{},
    };

    let info: SoftwareInfo = handle.unary(req).await?
        .result().await
        .unwrap();

    println!("{:#?}", info);

    let req = Request {
        address: packet_addr,
        channel_id: packet_addr.channel_id().unwrap(),
        service_id: Identifier::new("maestro_pw.Maestro").hash(),
        method_id: Identifier::new("SubscribeToSettingsChanges").hash(),
        call_id: 42,
        message: maestro::protocol::types::Empty{},
    };

    let mut call: Streaming<SettingsRsp> = handle.server_streaming(req).await?;
    while let Some(msg) = call.stream().next().await {
        println!("{:#?}", msg);
    }

    Ok(())
}

async fn run_client<S, E>(mut client: Client<S>)
where
    S: Sink<Packet>,
    S: futures::Stream<Item = Result<Packet, E>> + Unpin,
    S::Error: std::fmt::Debug,
    E: std::fmt::Debug,
{
    let result = client.run().await;

    if let Err(e) = result {
        log::error!("client shut down with error: {e:?}")
    }
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
