//! Simple example for receiving battery info via the GFPS RFCOMM channel.
//!
//! Usage:
//!   cargo run --example gfps_get_battery -- <bluetooth-device-address>

use std::str::FromStr;

use bluer::{Address, Session, Device};
use bluer::rfcomm::{Profile, ReqError, Role, ProfileHandle};

use futures::StreamExt;

use gfps::msg::{Codec, DeviceEventCode, EventGroup, BatteryInfo};

use num_enum::FromPrimitive;


#[tokio::main(flavor = "current_thread")]
async fn main() -> bluer::Result<()> {
    // handle command line arguments
    let addr = std::env::args().nth(1).expect("need device address as argument");
    let addr = Address::from_str(&addr)?;

    // set up session
    let session = Session::new().await?;
    let adapter = session.default_adapter().await?;

    // get device
    let dev = adapter.device(addr)?;

    // get RFCOMM stream
    let stream = {
        // register GFPS profile
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
        connect_device_to_profile(&mut profile_handle, &dev).await?
    };

    // listen to event messages
    let codec = Codec::new();
    let mut stream = codec.wrap(stream);

    // The battery status cannot be queried via a normal command. However, it
    // is sent right after we connect to the GFPS stream. In addition, multiple
    // events are often sent in sequence. Therefore we do the following:
    // - Set a deadline for a general timeout. If this passes, we just return
    //   the current state (and if necessary "unknown"):
    // - Use a timestamp for checking whether we have received any new updates
    //   in a given interval. If we have not received any, we consider the
    //   state to be "settled" and return the battery info.
    // - On battery events we simply store the sent information. We retreive
    //   the stored information once either of the timeouts kicks in.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);

    let mut timestamp = deadline;
    let mut bat_left = BatteryInfo::Unknown;
    let mut bat_right = BatteryInfo::Unknown;
    let mut bat_case = BatteryInfo::Unknown;

    let time_settle = std::time::Duration::from_millis(500);

    loop {
        tokio::select! {
            // receive and handle events
            msg = stream.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        let group = EventGroup::from_primitive(msg.group);
                        if group != EventGroup::Device {
                            continue;
                        }

                        let code = DeviceEventCode::from_primitive(msg.code);
                        if code == DeviceEventCode::BatteryInfo {
                            timestamp = std::time::Instant::now();

                            bat_left = BatteryInfo::from_byte(msg.data[0]);
                            bat_right = BatteryInfo::from_byte(msg.data[1]);
                            bat_case = BatteryInfo::from_byte(msg.data[2]);
                        }
                    },
                    Some(Err(err)) => {
                        Err(err)?;
                    },
                    None => {
                        let err = std::io::Error::new(
                            std::io::ErrorKind::ConnectionAborted,
                            "connection closed"
                        );

                        Err(err)?;
                    }
                }
            },
            // timeout for determining when the state has "settled"
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(time_settle.as_millis() as _)) => {
                let delta = std::time::Instant::now() - timestamp;

                if delta > time_settle {
                    break
                }
            },
            // general deadline
            _ = tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)) => {
                break
            },
        }
    }

    println!("Battery status:");
    println!("  left bud:  {}", bat_left);
    println!("  right bud: {}", bat_right);
    println!("  case:      {}", bat_case);

    Ok(())
}

async fn connect_device_to_profile(profile: &mut ProfileHandle, dev: &Device)
    -> bluer::Result<bluer::rfcomm::Stream>
{
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
            req = profile.next() => {
                let req = req.expect("no connection request received");

                if req.device() == dev.address() {
                    // accept our device
                    break req.accept();
                } else {
                    // reject unknown devices
                    req.reject(ReqError::Rejected);
                }
            },
        }
    }
}
