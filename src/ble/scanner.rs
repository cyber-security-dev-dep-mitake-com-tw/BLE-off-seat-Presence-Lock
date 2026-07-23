use anyhow::{Context, Result};
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use tokio::time::{Duration, sleep};

use super::os_paired::query_paired_devices;
use super::{BleScanResult, ScanCycle, device_matches};

/// Real BLE / OS-paired scanner backed by btleplug + platform paired-device APIs.
pub struct BtleScanner {
    adapter: Adapter,
}

impl BtleScanner {
    pub async fn new() -> Result<Self> {
        let manager = Manager::new()
            .await
            .context("failed to create BLE manager")?;
        let adapters = manager
            .adapters()
            .await
            .context("failed to list BLE adapters")?;
        let adapter = adapters
            .into_iter()
            .next()
            .context("no Bluetooth adapter found")?;
        Ok(Self { adapter })
    }

    pub async fn scan_once(
        &mut self,
        device_name: &str,
        whitelist: &[String],
    ) -> Result<ScanCycle> {
        // 1) OS-paired devices (classic + BLE) — matches README pairing model.
        let os = query_paired_devices(device_name, whitelist);
        let mut candidates = os.candidates;
        if let Some(reading) = os.reading {
            return Ok(ScanCycle {
                reading: Some(reading),
                candidates,
                source: "os-paired",
            });
        }

        // 2) BLE advertisement / GATT scan via btleplug.
        self.adapter
            .start_scan(ScanFilter::default())
            .await
            .context("failed to start BLE scan")?;

        sleep(Duration::from_millis(2000)).await;

        let peripherals = self
            .adapter
            .peripherals()
            .await
            .context("failed to enumerate peripherals")?;

        let mut best: Option<BleScanResult> = None;
        let mut ble_names = Vec::new();

        for peripheral in peripherals {
            let props = match peripheral.properties().await {
                Ok(Some(p)) => p,
                _ => continue,
            };

            let name = props.local_name.clone().filter(|n| !n.trim().is_empty());

            let Some(name) = name else {
                continue;
            };

            ble_names.push(name.clone());

            if !device_matches(&name, device_name, whitelist) {
                continue;
            }

            let display_name = name.clone();

            let rssi = if let Some(r) = props.rssi {
                r
            } else {
                // Connect and read RSSI when advertisement has no RSSI.
                if !peripheral.is_connected().await.unwrap_or(false) {
                    if let Err(e) = peripheral.connect().await {
                        log::debug!("BLE connect failed for {display_name}: {e}");
                        continue;
                    }
                }
                match peripheral.read_rssi().await {
                    Ok(r) => r,
                    Err(e) => {
                        log::debug!("read_rssi failed for {display_name}: {e}");
                        let _ = peripheral.disconnect().await;
                        continue;
                    }
                }
            };

            log::info!("[ble-scan] matched '{display_name}' rssi={rssi} dBm");

            let candidate = BleScanResult {
                device_name: display_name,
                rssi,
            };
            best = Some(match best {
                Some(prev) if prev.rssi >= candidate.rssi => prev,
                _ => candidate,
            });
        }

        if let Err(e) = self.adapter.stop_scan().await {
            log::debug!("stop_scan: {e}");
        }

        candidates.extend(ble_names);
        candidates.sort();
        candidates.dedup();

        if best.is_none() {
            log::info!(
                "no match for '{device_name}'. paired/seen devices: {}",
                if candidates.is_empty() {
                    "<none — check Bluetooth permission / pairing>".into()
                } else {
                    candidates.join(", ")
                }
            );
        }

        Ok(ScanCycle {
            reading: best,
            candidates,
            source: "ble-scan",
        })
    }
}
