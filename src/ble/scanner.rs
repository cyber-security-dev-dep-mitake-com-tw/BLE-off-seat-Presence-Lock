use anyhow::{Context, Result};
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use tokio::time::{Duration, sleep};

use super::{BleScanResult, device_matches};

/// Real BLE scanner backed by btleplug.
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
    ) -> Result<Option<BleScanResult>> {
        self.adapter
            .start_scan(ScanFilter::default())
            .await
            .context("failed to start BLE scan")?;

        // Allow advertisements to arrive.
        sleep(Duration::from_millis(1500)).await;

        let peripherals = self
            .adapter
            .peripherals()
            .await
            .context("failed to enumerate peripherals")?;

        let mut best: Option<BleScanResult> = None;

        for peripheral in peripherals {
            let props = match peripheral.properties().await {
                Ok(Some(p)) => p,
                _ => continue,
            };

            let Some(name) = props.local_name.as_deref() else {
                continue;
            };

            if !device_matches(name, device_name, whitelist) {
                continue;
            }

            // Prefer advertisement RSSI; fall back to read_rssi when connected.
            let rssi = if let Some(r) = props.rssi {
                r
            } else {
                match peripheral.read_rssi().await {
                    Ok(r) => r,
                    Err(e) => {
                        log::debug!("read_rssi failed for {name}: {e}");
                        continue;
                    }
                }
            };

            let candidate = BleScanResult {
                device_name: name.to_string(),
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
            // Not an error — device simply not in range / advertising.
            log::debug!("no matching BLE device found for '{device_name}'");
        }

        Ok(best)
    }
}
