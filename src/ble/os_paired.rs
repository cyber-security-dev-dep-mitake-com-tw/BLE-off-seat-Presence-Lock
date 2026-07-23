//! Query OS-paired Bluetooth devices (classic + BLE) for presence / RSSI.
//!
//! README promise: track devices already paired in system Bluetooth settings —
//! no per-device app registration required.

use crate::ble::{BleScanResult, device_matches};

#[derive(Debug, Clone, Default)]
pub struct OsDeviceSnapshot {
    /// Best match for the configured target, if any.
    pub reading: Option<BleScanResult>,
    /// Human-readable names of paired (and optionally nearby) devices.
    pub candidates: Vec<String>,
}

/// Look up paired/connected devices via the platform Bluetooth stack.
pub fn query_paired_devices(device_name: &str, whitelist: &[String]) -> OsDeviceSnapshot {
    #[cfg(target_os = "macos")]
    {
        macos::query(device_name, whitelist)
    }
    #[cfg(target_os = "windows")]
    {
        windows::query(device_name, whitelist)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (device_name, whitelist);
        OsDeviceSnapshot::default()
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use objc2_foundation::NSString;
    use objc2_io_bluetooth::IOBluetoothDevice;

    const RSSI_UNAVAILABLE: i8 = 127;

    pub fn query(device_name: &str, whitelist: &[String]) -> OsDeviceSnapshot {
        let mut candidates = Vec::new();
        let mut reading: Option<BleScanResult> = None;

        let Some(devices) = (unsafe { IOBluetoothDevice::pairedDevices() }) else {
            log::warn!(
                "IOBluetooth pairedDevices returned nil — grant Bluetooth permission if prompted"
            );
            return OsDeviceSnapshot {
                reading: None,
                candidates,
            };
        };

        let count = devices.count();
        log::debug!("IOBluetooth paired device count={count}");

        for i in 0..count {
            let obj = devices.objectAtIndex(i);
            let Some(device) = obj.downcast_ref::<IOBluetoothDevice>() else {
                continue;
            };

            let name = device_display_name(device);
            if name.is_empty() {
                continue;
            }
            candidates.push(name.clone());

            if !device_matches(&name, device_name, whitelist) {
                continue;
            }

            if let Some(rssi) = read_rssi(device) {
                let candidate = BleScanResult {
                    device_name: name.clone(),
                    rssi,
                };
                log::info!(
                    "[os-paired] matched '{}' connected={} rssi={} dBm",
                    name,
                    unsafe { device.isConnected() },
                    rssi
                );
                reading = Some(match reading {
                    Some(prev) if prev.rssi >= candidate.rssi => prev,
                    _ => candidate,
                });
            } else {
                log::info!(
                    "[os-paired] matched '{}' but RSSI unavailable (away or not connectable)",
                    name
                );
            }
        }

        candidates.sort();
        candidates.dedup();

        OsDeviceSnapshot {
            reading,
            candidates,
        }
    }

    fn device_display_name(device: &IOBluetoothDevice) -> String {
        unsafe {
            if let Some(n) = device.nameOrAddress() {
                return n.to_string();
            }
            device.name().to_string()
        }
    }

    fn read_rssi(device: &IOBluetoothDevice) -> Option<i16> {
        unsafe {
            // Only sample RSSI for already-connected devices.
            // openConnection() is synchronous and can hang for minutes on
            // out-of-range paired devices, so we never initiate connects here.
            if !device.isConnected() {
                return None;
            }

            let raw = device.rawRSSI();
            if raw != RSSI_UNAVAILABLE {
                return Some(i16::from(raw));
            }

            // Fall back to relative RSSI (0 = golden range).
            let rel = device.RSSI();
            if rel != RSSI_UNAVAILABLE {
                // Map relative golden-range offset into a plausible dBm-ish value.
                return Some(i16::from(rel).saturating_sub(50));
            }

            // Connected but RSSI unreadable — still treat as present/nearby.
            Some(-55)
        }
    }

    // Keep NSString in scope for type inference helpers if needed.
    #[allow(dead_code)]
    fn _touch_nsstring(s: &NSString) -> String {
        s.to_string()
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use super::*;

    /// Windows paired-device RSSI still primarily comes from BLE advertisements
    /// (btleplug). Here we at least surface PnP Bluetooth device names so the UI
    /// can show what the OS knows about.
    pub fn query(device_name: &str, whitelist: &[String]) -> OsDeviceSnapshot {
        let mut candidates = Vec::new();
        let output = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-PnpDevice -Class Bluetooth -Status OK -ErrorAction SilentlyContinue | Select-Object -ExpandProperty FriendlyName",
            ])
            .output();

        let Ok(output) = output else {
            return OsDeviceSnapshot::default();
        };
        if !output.status.success() {
            return OsDeviceSnapshot::default();
        }

        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            let name = line.trim();
            if name.is_empty() {
                continue;
            }
            candidates.push(name.to_string());
        }
        candidates.sort();
        candidates.dedup();

        // No reliable RSSI from PnP listing alone.
        let reading = candidates
            .iter()
            .find(|n| device_matches(n, device_name, whitelist))
            .map(|n| {
                log::info!(
                    "[os-paired] found Windows Bluetooth device '{}' (RSSI via BLE scan)",
                    n
                );
                // Return None reading — BLE scanner will supply RSSI.
                n
            });
        let _ = reading;

        OsDeviceSnapshot {
            reading: None,
            candidates,
        }
    }
}
