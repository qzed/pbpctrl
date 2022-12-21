//! Simple example for listening to Maestro messages sent via the RFCOMM channel.
//!
//! Usage:
//!   cargo run --example maestro_listen -- <bluetooth-device-address>

use std::collections::{HashSet, HashMap};
use std::str::FromStr;

use bluer::{Address, Session, Device};
use bluer::rfcomm::{Profile, ReqError, Role, ProfileHandle};

use futures::StreamExt;

use maestro::pwrpc::id::{self, Identifier};
use maestro::pwrpc::codec::{Codec, Packet};
use maestro::pwrpc::types::PacketType;
use maestro::protocol;
use prost::Message;


#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    env_logger::init();

    // packet printer
    let printer = Printer::create();

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

        while let Some(packet) = stream.next().await {
            match packet {
                Ok(packet) => {
                    printer.handle(&packet);
                    println!()
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


struct Printer {
    map: HashMap<(id::Hash, id::Hash), &'static dyn Fn(&Packet) -> ()>,
}

impl Printer {
    fn create() -> Self {
        let mut map = HashMap::<_, &'static dyn Fn(&Packet) -> ()>::new();

        let id_maestro = Identifier::new("maestro_pw.Maestro");
        let id_swinfo = Identifier::new("GetSoftwareInfo");

        map.insert((id_maestro.hash(), id_swinfo.hash()), &print_packet_swinfo);

        Self { map }
    }

    fn handle(&self, packet: &Packet) {
        match self.map.get(&(packet.rpc.service_id, packet.rpc.method_id)) {
            Some(handler) => handler(packet),
            None => print_packet_default(packet)
        }
    }
}

fn print_packet_swinfo(packet: &Packet) {
    println!("Frame:");
    println!("  address: 0x{:04x}", packet.address);
    println!("  packet:");
    println!("    type:    {:?}", PacketType::from_i32(packet.rpc.r#type).unwrap());
    println!("    channel: {:?}", packet.rpc.channel_id);
    println!("    service: maestro_pw.Maestro");
    println!("    method:  GetSoftwareInfo");
    println!("    status:  {:?}", packet.rpc.status);
    println!("    call-id: {:?}", packet.rpc.call_id);
    println!("    payload:");

    let info = protocol::types::SoftwareInfo::decode(&packet.rpc.payload[..])
        .expect("failed to decode SoftwareInfo packet");

    println!("{:#02x?}", info);
}

fn print_packet_default(packet: &Packet) {
    println!("Frame:");
    println!("  address: 0x{:04x}", packet.address);
    println!("  packet:");
    println!("    type:    {:?}", PacketType::from_i32(packet.rpc.r#type).unwrap());
    println!("    channel: {:?}", packet.rpc.channel_id);
    println!("    service: {:08x?}", packet.rpc.service_id);
    println!("    method:  {:08x?}", packet.rpc.method_id);
    println!("    status:  {:?}", packet.rpc.status);
    println!("    call-id: {:?}", packet.rpc.call_id);
    println!("    payload:");

    let data = pretty_hex::config_hex(
        &packet.rpc.payload,
        pretty_hex::HexConfig {
            title: false,
            ..Default::default()
        },
    );

    for line in data.lines() {
        println!("      {}", line);
    }
}
