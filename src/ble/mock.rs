use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

use super::{BleScanResult, ScanCycle, device_matches};

/// Mock BLE scanner for testing without hardware.
///
/// RSSI sources (priority order):
/// 1. `debug_mock_file` contents (single integer dBm)
/// 2. Built-in default strong signal (-50 dBm)
pub struct MockScanner {
    mock_file: Option<PathBuf>,
    default_rssi: i16,
    tick: u64,
}

impl MockScanner {
    pub fn new(mock_file: Option<String>) -> Self {
        Self {
            mock_file: mock_file.map(PathBuf::from),
            default_rssi: -50,
            tick: 0,
        }
    }

    pub async fn scan_once(
        &mut self,
        device_name: &str,
        whitelist: &[String],
    ) -> Result<ScanCycle> {
        self.tick = self.tick.wrapping_add(1);

        let name = if !whitelist.is_empty() {
            whitelist[0].clone()
        } else {
            device_name.to_string()
        };

        if !device_matches(&name, device_name, whitelist) {
            return Ok(ScanCycle {
                reading: None,
                candidates: vec![name],
                source: "mock",
            });
        }

        let rssi = self.read_rssi()?;
        log::info!(
            "[mock] scan tick={} device='{}' rssi={} dBm",
            self.tick,
            name,
            rssi
        );

        Ok(ScanCycle {
            reading: Some(BleScanResult {
                device_name: name.clone(),
                rssi,
            }),
            candidates: vec![name],
            source: "mock",
        })
    }

    fn read_rssi(&self) -> Result<i16> {
        if let Some(path) = &self.mock_file {
            if path.exists() {
                let raw = fs::read_to_string(path)
                    .with_context(|| format!("failed to read mock RSSI file {}", path.display()))?;
                let trimmed = raw.trim();
                if !trimmed.is_empty() {
                    let value: i16 = trimmed
                        .parse()
                        .with_context(|| format!("invalid RSSI in mock file: '{trimmed}'"))?;
                    return Ok(value);
                }
            }
        }
        Ok(self.default_rssi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn returns_default_rssi() {
        let mut s = MockScanner::new(None);
        let cycle = s.scan_once("iPhone", &[]).await.unwrap();
        let r = cycle.reading.unwrap();
        assert_eq!(r.rssi, -50);
        assert_eq!(r.device_name, "iPhone");
    }

    #[tokio::test]
    async fn reads_rssi_from_file() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "-88\n").unwrap();
        let mut s = MockScanner::new(Some(f.path().display().to_string()));
        let r = s.scan_once("iPhone", &[]).await.unwrap().reading.unwrap();
        assert_eq!(r.rssi, -88);
    }
}
