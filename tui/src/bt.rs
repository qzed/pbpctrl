use std::time::Duration;

use anyhow::Result;

use bluer::{Adapter, Address, Device, Session};
use bluer::rfcomm::{ProfileHandle, Role, ReqError, Stream, Profile};

use futures::StreamExt;


const PIXEL_BUDS_CLASS: u32 = 0x240404;
const PIXEL_BUDS2_CLASS: u32 = 0x244404;


pub async fn find_maestro_device(adapter: &Adapter) -> Result<Device> {
    for addr in adapter.device_addresses().await? {
        let dev = adapter.device(addr)?;

        let class = dev.class().await?.unwrap_or(0);
        if class != PIXEL_BUDS_CLASS && class != PIXEL_BUDS2_CLASS {
            continue;
        }

        let uuids = dev.uuids().await?.unwrap_or_default();
        if !uuids.contains(&maestro::UUID) {
            continue;
        }

        tracing::debug!(address=%addr, "found compatible device");
        return Ok(dev);
    }

    tracing::debug!("no compatible device found");
    anyhow::bail!("no compatible device found")
}

pub async fn connect_maestro_rfcomm(session: &Session, dev: &Device) -> Result<Stream> {
    let maestro_profile = Profile {
        uuid: maestro::UUID,
        role: Some(Role::Client),
        require_authentication: Some(false),
        require_authorization: Some(false),
        auto_connect: Some(false),
        ..Default::default()
    };

    tracing::debug!("registering maestro profile");
    let mut handle = session.register_profile(maestro_profile).await?;

    tracing::debug!("connecting to maestro profile");
    let stream = tokio::try_join!(
        try_connect_profile(dev),
        handle_requests_for_profile(&mut handle, dev.address()),
    )?.1;

    Ok(stream)
}

async fn try_connect_profile(dev: &Device) -> Result<()> {
    const RETRY_TIMEOUT: Duration = Duration::from_secs(1);
    const MAX_TRIES: u32 = 3;

    let mut i = 0;
    while let Err(err) = dev.connect_profile(&maestro::UUID).await {
        if i >= MAX_TRIES { return Err(err.into()) }
        i += 1;

        tracing::debug!(error=?err, "connecting to profile failed, trying again ({}/{})", i, MAX_TRIES);

        tokio::time::sleep(RETRY_TIMEOUT).await;
    }

    tracing::debug!(address=%dev.address(), "maestro profile connected");
    Ok(())
}

async fn handle_requests_for_profile(handle: &mut ProfileHandle, address: Address) -> Result<Stream> {
    while let Some(req) = handle.next().await {
        tracing::debug!(address=%req.device(), "received new profile connection request");

        if req.device() == address {
            tracing::debug!(address=%req.device(), "accepting profile connection request");
            return Ok(req.accept()?);
        } else {
            req.reject(ReqError::Rejected);
        }
    }

    anyhow::bail!("profile terminated without requests")
}
