//! Simple example for reading battery info via the Maestro service.
//!
//! Usage:
//!   cargo run --example maestro_get_battery -- <bluetooth-device-address>

use std::str::FromStr;

use anyhow::bail;

use bluer::{Address, Session, Device};
use bluer::rfcomm::{Profile, ReqError, Role, ProfileHandle};

use futures::{StreamExt, Sink};

use maestro::protocol::codec::Codec;
use maestro::protocol::types::RuntimeInfo;
use maestro::pwrpc::client::{Client, ClientHandle};
use maestro::pwrpc::types::RpcPacket;
use maestro::pwrpc::Error;
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

    let mut call = service.subscribe_to_runtime_info().await?;
    let rt_info = if let Some(msg) = call.stream().next().await {
        msg?
    } else {
        bail!("did not receive any runtime-info event");
    };

    call.cancel_and_wait().await?;
    Ok(rt_info)
}

async fn run_client<S, E>(mut client: Client<S>) -> anyhow::Result<()>
where
    S: Sink<RpcPacket>,
    S: futures::Stream<Item = Result<RpcPacket, E>> + Unpin,
    Error: From<E>,
    Error: From<S::Error>,
{
    tokio::select! {
        res = client.run() => {
            res?;
        },
        sig = tokio::signal::ctrl_c() => {
            sig?;
            tracing::trace!("client termination requested");
        },
    }

    client.terminate().await?;
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
