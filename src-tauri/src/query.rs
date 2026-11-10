//! Opt-in GATT query: connect to one device, read a few standard
//! characteristics, and disconnect. Works the same on every platform.

use std::time::Duration;

use btleplug::api::bleuuid::uuid_from_u16;
use btleplug::api::{Central, CharPropFlags, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Manager, Peripheral};
use tokio::time::{sleep, timeout};

use crate::decode;
use crate::device::GattInfo;
use crate::store::now_ms;

const OVERALL_TIMEOUT: Duration = Duration::from_secs(45);
const FIND_TIMEOUT: Duration = Duration::from_secs(15);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const READ_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn query_device(id: &str) -> Result<GattInfo, String> {
    timeout(OVERALL_TIMEOUT, run(id))
        .await
        .map_err(|_| "The query timed out".to_string())?
}

async fn run(id: &str) -> Result<GattInfo, String> {
    let manager = Manager::new().await.map_err(|e| e.to_string())?;
    let adapter = manager
        .adapters()
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .next()
        .ok_or("No Bluetooth adapter found")?;

    // The peripheral has to be (re)discovered by this adapter before we can connect.
    adapter
        .start_scan(ScanFilter::default())
        .await
        .map_err(|e| e.to_string())?;
    let peripheral = timeout(FIND_TIMEOUT, async {
        loop {
            if let Ok(found) = adapter.peripherals().await {
                if let Some(p) = found
                    .into_iter()
                    .find(|p| p.id().to_string().to_lowercase() == id)
                {
                    return p;
                }
            }
            sleep(Duration::from_millis(250)).await;
        }
    })
    .await
    .map_err(|_| "The device was not found nearby. Is it still advertising?".to_string());
    let _ = adapter.stop_scan().await;
    let peripheral = peripheral?;

    timeout(CONNECT_TIMEOUT, peripheral.connect())
        .await
        .map_err(|_| "Connecting timed out".to_string())?
        .map_err(|e| format!("Could not connect: {e}"))?;

    let info = read_info(&peripheral).await;
    let _ = peripheral.disconnect().await;
    info
}

fn text(bytes: Vec<u8>) -> Option<String> {
    let s = String::from_utf8_lossy(&bytes)
        .trim_matches(char::from(0))
        .trim()
        .to_string();
    (!s.is_empty()).then_some(s)
}

async fn read_info(peripheral: &Peripheral) -> Result<GattInfo, String> {
    peripheral
        .discover_services()
        .await
        .map_err(|e| format!("Could not list services: {e}"))?;

    let mut info = GattInfo {
        queried_at: now_ms(),
        ..Default::default()
    };
    for characteristic in peripheral.characteristics() {
        if !characteristic.properties.contains(CharPropFlags::READ) {
            continue;
        }
        let uuid = characteristic.uuid;
        let wanted = [
            0x2a00, 0x2a01, 0x2a19, 0x2a24, 0x2a26, 0x2a27, 0x2a28, 0x2a29,
        ]
        .into_iter()
        .find(|short| uuid == uuid_from_u16(*short));
        let Some(short) = wanted else { continue };

        let Ok(Ok(bytes)) = timeout(READ_TIMEOUT, peripheral.read(&characteristic)).await else {
            continue;
        };
        match short {
            0x2a00 if info.device_name.is_none() => info.device_name = text(bytes),
            0x2a01 if info.appearance.is_none() && bytes.len() >= 2 => {
                let value = u16::from_le_bytes([bytes[0], bytes[1]]);
                info.appearance = Some(value);
                info.appearance_label = decode::appearance_label(value);
            }
            0x2a19 if info.battery_percent.is_none() => info.battery_percent = bytes.first().copied(),
            0x2a24 if info.model_number.is_none() => info.model_number = text(bytes),
            0x2a26 if info.firmware_revision.is_none() => info.firmware_revision = text(bytes),
            0x2a27 if info.hardware_revision.is_none() => info.hardware_revision = text(bytes),
            0x2a28 if info.software_revision.is_none() => info.software_revision = text(bytes),
            0x2a29 if info.manufacturer_name.is_none() => info.manufacturer_name = text(bytes),
            _ => {}
        }
    }
    Ok(info)
}
