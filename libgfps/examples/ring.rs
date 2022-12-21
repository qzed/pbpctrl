//! Simple example for "ringing" the buds to locate them.
//!
//! WARNING: DO NOT RUN THIS EXAMPLE WITH THE BUDS IN YOUR EAR! YOU HAVE BEEN WARNED.
//!
//! Usage:
//!   cargo run --example ring -- <bluetooth-device-address>

use std::str::FromStr;

use bluer::{Address, Session, Device};
use bluer::rfcomm::{Profile, Role, ProfileHandle, ReqError};

use futures::{StreamExt, SinkExt};

use gfps::msg::{Codec, Message, EventGroup, DeviceActionEventCode, AcknowledgementEventCode};

use num_enum::FromPrimitive;

use smallvec::smallvec;


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

    // set up message stream
    let codec = Codec::new();
    let mut stream = codec.wrap(stream);

    // send "ring" message
    //
    // Note: Pixel Buds Pro ignore messages with a timeout. So don't specify
    // one here.
    let msg = Message {
        group: EventGroup::DeviceAction.into(),
        code: DeviceActionEventCode::Ring.into(),
        data: smallvec![0x03],      // 0b01: right, 0b10: left, 0b10|0b01 = 0b11: both
    };

    println!("Ringing buds...");
    stream.send(&msg).await?;

    // An ACK message should come in 1s. Wait for that.
    let timeout = tokio::time::Instant::now() + tokio::time::Duration::from_secs(1);
    loop {
        tokio::select! {
            msg = stream.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        println!("{:?}", msg);

                        let group = EventGroup::from_primitive(msg.group);
                        if group != EventGroup::Acknowledgement {
                            continue;
                        }

                        let ack_group = EventGroup::from_primitive(msg.data[0]);
                        if ack_group != EventGroup::DeviceAction {
                            continue;
                        }

                        let ack_code = DeviceActionEventCode::from_primitive(msg.data[1]);
                        if ack_code != DeviceActionEventCode::Ring {
                            continue;
                        }

                        let code = AcknowledgementEventCode::from_primitive(msg.code);

                        if code == AcknowledgementEventCode::Ack {
                            println!("Received ACK for ring command");
                            break;

                        } else if code == AcknowledgementEventCode::Nak {
                            println!("Received NAK for ring command");

                            let err = std::io::Error::new(
                                std::io::ErrorKind::Unsupported,
                                "ring has been NAK'ed by device"
                            );

                            Err(err)?;
                        }
                    },
                    Some(Err(e)) => {
                        Err(e)?;
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
            _ = tokio::time::sleep_until(timeout) => {
                let err = std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "timed out, ring action might be unsupported"
                );

                Err(err)?;
            },
        }
    }

    // Next, the device will communicate back status updates. This may include
    // an initial update to confirm ringing and follow-up updates once the user
    // has touched the buds and ringing stops.
    //
    // Stop this program once we have no more rining or once we have reached a
    // timeout of 30s.
    let mut timeout = tokio::time::Instant::now() + tokio::time::Duration::from_secs(30);
    loop {
        tokio::select! {
            msg = stream.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        println!("{:?}", msg);

                        let group = EventGroup::from_primitive(msg.group);
                        if group != EventGroup::DeviceAction {
                            continue;
                        }
                        // send ACK
                        let ack = Message {
                            group: EventGroup::Acknowledgement.into(),
                            code: AcknowledgementEventCode::Ack.into(),
                            data: smallvec![msg.group, msg.code],
                        };

                        stream.send(&ack).await?;

                        let status = msg.data[0];

                        println!("Received ring update:");

                        if status & 0b01 != 0 {
                            println!("  right: ringing");
                        } else {
                            println!("  right: not ringing");
                        }

                        if status & 0b10 != 0 {
                            println!("  left:  ringing");
                        } else {
                            println!("  left:  not ringing");
                        }

                        if status & 0b11 == 0 {
                            println!("Buds stopped ringing, exiting...");
                            return Ok(());
                        }
                    },
                    Some(Err(e)) => {
                        Err(e)?;
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
            _ = tokio::time::sleep_until(timeout) => {
                println!("Sending command to stop ringing...");

                // send message to stop ringing
                let msg = Message {
                    group: EventGroup::DeviceAction.into(),
                    code: DeviceActionEventCode::Ring.into(),
                    data: smallvec![0x00],
                };

                stream.send(&msg).await?;

                timeout = tokio::time::Instant::now() + tokio::time::Duration::from_secs(10);
            },
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
