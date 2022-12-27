//! Simple example for listening to Maestro messages sent via the RFCOMM channel.
//!
//! Usage:
//!   cargo run --example maestro_listen -- <bluetooth-device-address>

mod common;

use std::str::FromStr;

use bluer::{Address, Session};
use futures::StreamExt;

use maestro::protocol::codec::Codec;
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

    // try to reconnect if connection is reset
    loop {
        println!("Connecting to Maestro profile");
        let stream = common::connect_maestro_rfcomm(&session, &dev).await?;

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

        let exec_task = common::run_client(client);
        let listen_task = run_listener(handle, channel);

        tokio::select! {
            res = exec_task => {
                match res {
                    Ok(_) => {
                        tracing::trace!("client terminated successfully");
                        return Ok(());
                    },
                    Err(e) => {
                        tracing::error!("client task terminated with error");

                        let cause = e.root_cause();
                        if let Some(cause) = cause.downcast_ref::<std::io::Error>() {
                            if cause.raw_os_error() == Some(104) {
                                // The Pixel Buds Pro can hand off processing between each
                                // other. On a switch, the connection is reset. Wait a bit
                                // and then try to reconnect.
                                println!();
                                println!("Connection reset. Attempting to reconnect...");
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                continue;
                            }
                        }

                        return Err(e);
                    },
                }
            },
            res = listen_task => {
                match res {
                    Ok(_) => {
                        tracing::error!("server terminated stream");
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::error!("main task terminated with error");
                        return Err(e);
                    }
                }
            },
        }
    }
}

async fn run_listener(handle: ClientHandle, channel: u32) -> anyhow::Result<()> {
    println!("Sending GetSoftwareInfo request");
    println!();

    let mut service = MaestroService::new(handle, channel);

    let info = service.get_software_info().await?;
    println!("{:#?}", info);

    println!();
    println!("Listening to settings changes...");
    println!();

    let task_rtinfo = run_listener_rtinfo(service.clone());
    let task_settings = run_listener_settings(service.clone());

    tokio::select! {
        res = task_rtinfo => res,
        res = task_settings => res,
    }
}

async fn run_listener_rtinfo(mut service: MaestroService) -> anyhow::Result<()> {
    let mut call = service.subscribe_to_runtime_info()?;
    while let Some(msg) = call.stream().next().await {
        println!("{:#?}", msg?);
    }

    Ok(())
}

async fn run_listener_settings(mut service: MaestroService) -> anyhow::Result<()> {
    let mut call = service.subscribe_to_settings_changes()?;
    while let Some(msg) = call.stream().next().await {
        println!("{:#?}", msg?);
    }

    Ok(())
}
