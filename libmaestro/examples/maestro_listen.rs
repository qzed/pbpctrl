//! Simple example for listening to Maestro messages sent via the RFCOMM channel.
//!
//! Usage:
//!   cargo run --example maestro_listen -- <bluetooth-device-address>

use std::str::FromStr;

use bluer::{Address, Session, Device};
use bluer::rfcomm::{Profile, ReqError, Role, ProfileHandle};

use futures::{StreamExt, Sink};

use maestro::pwrpc::client::Client;
use maestro::pwrpc::id::Identifier;
use maestro::pwrpc::codec::{Codec, Packet};
use maestro::pwrpc::types::{RpcType, PacketType, RpcPacket};
use maestro::protocol::{self, addr};

use prost::Message;


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
                Err(e)?
            }
        }
    }

    let client = Client::new(stream);
    let handle = client.handle();

    tokio::spawn(run_client(client));

    let mut call = handle.call(RpcType::Unary, Packet {
        address: packet_addr.value(),
        rpc: RpcPacket {
            r#type: PacketType::Request as _,
            channel_id: packet_addr.channel_id().unwrap(),
            service_id: Identifier::new("maestro_pw.Maestro").hash(),
            method_id: Identifier::new("GetSoftwareInfo").hash(),
            payload: maestro::protocol::types::Empty{}.encode_to_vec(),
            status: 0,
            call_id: 42,
        },
    }).await?;

    let response = call.result().await.unwrap();
    {
        let info = protocol::types::SoftwareInfo::decode(&response[..])
            .expect("failed to decode SoftwareInfo packet");

        println!("{:#?}", info);
    }

    let mut call = handle.call(RpcType::ServerStream, Packet {
        address: packet_addr.value(),
        rpc: RpcPacket {
            r#type: PacketType::Request as _,
            channel_id: packet_addr.channel_id().unwrap(),
            service_id: Identifier::new("maestro_pw.Maestro").hash(),
            method_id: Identifier::new("SubscribeToSettingsChanges").hash(),
            payload: maestro::protocol::types::Empty{}.encode_to_vec(),
            status: 0,
            call_id: 42,
        },
    }).await?;

    let mut changes = call.stream();
    while let Some(data) = changes.next().await {
        let data = data.unwrap();

        let info = protocol::types::SettingsRsp::decode(&data[..])
            .expect("failed to decode SettingsRsp packet");

        println!("{:#?}", info);
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
