//! Simple example for listening to GFPS messages sent via the RFCOMM channel.
//!
//! Usage:
//!   cargo run --example listen -- <bluetooth-device-address>

use std::collections::HashSet;
use std::str::FromStr;

use bluer::{Address, Session};
use bluer::rfcomm::{Profile, Role, ReqError};

use futures::StreamExt;

use gfps::msg::Codec;


#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    // handle command line arguments
    let addr = std::env::args().skip(1).next().expect("need device address as argument");
    let addr = Address::from_str(&addr)?;

    // set up session
    let session = Session::new().await?;
    let adapter = session.default_adapter().await?;

    println!("Using adapter '{}'", adapter.name());

    // get device
    let dev = adapter.device(addr)?;
    let uuids = {
        let mut uuids = Vec::from_iter(dev.uuids().await?
            .unwrap_or_else(HashSet::new)
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
        let stream = {
            // register GFPS profile
            println!("Registering GFPS profile...");

            let profile = Profile {
                uuid: gfps::msg::UUID,
                role: Some(Role::Client),
                require_authentication: Some(false),
                require_authorization: Some(false),
                auto_connect: Some(false),
                ..Default::default()
            };

            let mut profile_handle = session.register_profile(profile).await?;

            // connect profile
            println!("Connecting GFPS profile...");

            loop {
                tokio::select! {
                    res = async {
                        let _ = dev.connect().await;
                        dev.connect_profile(&gfps::msg::UUID).await
                    } => {
                        if let Err(err) = res {
                            println!("Connecting GFPS profile failed: {:?}", err);
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(3000)).await;
                    },
                    req = profile_handle.next() => {
                        let req = req.expect("no connection request received");

                        if req.device() == addr {
                            println!("Accepting request...");
                            break req.accept()?;
                        } else {
                            println!("Rejecting unknown device {}", req.device());
                            req.reject(ReqError::Rejected);
                        }
                    },
                }
            }
        };

        println!("Profile connected");

        // listen to event messages
        let codec = Codec::new();
        let mut stream = codec.wrap(stream);

        println!("Listening...");
        println!();

        while let Some(msg) = stream.next().await {
            match msg {
                Ok(msg) => {
                    println!("{:?}", msg);
                },
                Err(e) if e.raw_os_error() == Some(104) => {
                    // The Pixel Buds Pro can hand off processing between each
                    // other. On a switch, the connection is reset. Wait a bit
                    // and then try to reconnect.
                    println!();
                    println!("Connection reset. Attempting to reconnect...");
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    break;
                },
                Err(e) => {
                    Err(e)?;
                }
            }
        }
    }
}
