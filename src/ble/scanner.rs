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
        // Always collect OS-paired names for the UI / config hints.
        let os = query_paired_devices(device_name, whitelist);
        let mut candidates = os.candidates;

        // Prefer BLE advertisement RSSI for presence.
        // "Not Connected" in System Settings only means no audio/data profile
        // link — the device can still be nearby and advertising.
        let ble = self.scan_ble(device_name, whitelist).await?;
        candidates.extend(ble.candidates);
        candidates.sort();
        candidates.dedup();

        if let Some(reading) = ble.reading {
            return Ok(ScanCycle {
                reading: Some(reading),
                candidates,
                source: "ble-scan",
            });
        }

        // Fallback: classic OS-connected link with a valid negative raw RSSI.
        if let Some(reading) = os.reading {
            return Ok(ScanCycle {
                reading: Some(reading),
                candidates,
                source: "os-paired",
            });
        }

        Ok(ScanCycle {
            reading: None,
            candidates,
            source: "none",
        })
    }

    async fn scan_ble(
        &mut self,
        device_name: &str,
        whitelist: &[String],
    ) -> Result<ScanCycle> {
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

            let Some(name) = props.local_name.clone().filter(|n| !n.trim().is_empty()) else {
                continue;
            };

            ble_names.push(name.clone());

            if !device_matches(&name, device_name, whitelist) {
                continue;
            }

            // Presence uses advertisement RSSI only — do not GATT-connect.
            // Connecting would create an LE link that confuses "Connected" state.
            let Some(rssi) = props.rssi else {
                log::debug!("BLE device '{name}' has no advertisement RSSI yet");
                continue;
            };

            log::info!("[ble-scan] matched '{name}' rssi={rssi} dBm (advertisement)");

            let candidate = BleScanResult {
                device_name: name,
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

        if best.is_none() {
            log::info!(
                "no BLE advertisement match for '{device_name}'. seen names: {}",
                if ble_names.is_empty() {
                    "<none>".into()
                } else {
                    {
                        let mut n = ble_names.clone();
                        n.sort();
                        n.dedup();
                        n.join(", ")
                    }
                }
            );
        }

        Ok(ScanCycle {
            reading: best,
            candidates: ble_names,
            source: "ble-scan",
        })
    }
}
