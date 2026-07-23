pub mod mock;
pub mod os_paired;
pub mod scanner;

use anyhow::Result;

use self::mock::MockScanner;
use self::scanner::BtleScanner;

/// One RSSI observation from a scan cycle.
#[derive(Debug, Clone)]
pub struct BleScanResult {
    pub device_name: String,
    pub rssi: i16,
}

/// Result of one monitoring poll, including discovery hints for the UI.
#[derive(Debug, Clone)]
pub struct ScanCycle {
    pub reading: Option<BleScanResult>,
    pub candidates: Vec<String>,
    pub source: &'static str,
}

/// Scanner backend selected at startup (`--mock` or real BLE/OS).
pub enum Scanner {
    Real(BtleScanner),
    Mock(MockScanner),
}

impl Scanner {
    pub async fn real() -> Result<Self> {
        Ok(Self::Real(BtleScanner::new().await?))
    }

    pub fn mock(mock_file: Option<String>) -> Self {
        Self::Mock(MockScanner::new(mock_file))
    }

    /// Perform one scan cycle.
    pub async fn scan_once(
        &mut self,
        device_name: &str,
        whitelist: &[String],
    ) -> Result<ScanCycle> {
        match self {
            Self::Real(s) => s.scan_once(device_name, whitelist).await,
            Self::Mock(s) => s.scan_once(device_name, whitelist).await,
        }
    }
}

pub(crate) fn device_matches(name: &str, device_name: &str, whitelist: &[String]) -> bool {
    if !whitelist.is_empty() {
        return whitelist
            .iter()
            .any(|w| w.eq_ignore_ascii_case(name));
    }
    name.to_ascii_lowercase()
        .contains(&device_name.to_ascii_lowercase())
}
