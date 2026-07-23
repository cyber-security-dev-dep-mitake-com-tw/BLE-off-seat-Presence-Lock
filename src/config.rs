use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse config YAML: {0}")]
    Parse(#[from] serde_yaml::Error),
    #[error("invalid config: {0}")]
    Validation(String),
}

/// Application configuration loaded from YAML.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// BLE device name to track (as shown in OS Bluetooth settings).
    pub device_name: String,

    /// Lock when smoothed RSSI drops below this value (dBm).
    #[serde(default = "default_rssi_threshold")]
    pub rssi_threshold: i16,

    /// Seconds RSSI must stay below threshold before locking.
    #[serde(default = "default_grace_period")]
    pub grace_period: u64,

    /// EMA smoothing factor in \[0.0, 1.0\].
    #[serde(default = "default_ema_alpha")]
    pub ema_alpha: f64,

    /// Seconds between BLE scan / RSSI read cycles.
    #[serde(default = "default_scan_interval")]
    pub scan_interval: u64,

    /// Log level: debug / info / warn / error.
    #[serde(default = "default_log_level")]
    pub log_level: String,

    /// Start monitoring automatically on launch.
    #[serde(default = "default_auto_start")]
    pub auto_start: bool,

    /// Optional whitelist; if non-empty, only these names are tracked.
    #[serde(default)]
    pub device_whitelist: Vec<String>,

    /// Optional file path for mock RSSI values (one integer dBm per line / overwrite).
    #[serde(default)]
    pub debug_mock_file: Option<String>,
}

fn default_rssi_threshold() -> i16 {
    -80
}
fn default_grace_period() -> u64 {
    10
}
fn default_ema_alpha() -> f64 {
    0.2
}
fn default_scan_interval() -> u64 {
    2
}
fn default_log_level() -> String {
    "info".into()
}
fn default_auto_start() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            device_name: "iPhone".into(),
            rssi_threshold: default_rssi_threshold(),
            grace_period: default_grace_period(),
            ema_alpha: default_ema_alpha(),
            scan_interval: default_scan_interval(),
            log_level: default_log_level(),
            auto_start: default_auto_start(),
            device_whitelist: Vec::new(),
            debug_mock_file: None,
        }
    }
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&contents)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.device_name.trim().is_empty() {
            return Err(ConfigError::Validation(
                "device_name must not be empty".into(),
            ));
        }
        if !(0.0..=1.0).contains(&self.ema_alpha) {
            return Err(ConfigError::Validation(format!(
                "ema_alpha must be in [0.0, 1.0], got {}",
                self.ema_alpha
            )));
        }
        if self.grace_period == 0 {
            return Err(ConfigError::Validation(
                "grace_period must be greater than 0".into(),
            ));
        }
        if self.scan_interval == 0 {
            return Err(ConfigError::Validation(
                "scan_interval must be greater than 0".into(),
            ));
        }
        let level = self.log_level.to_ascii_lowercase();
        if !matches!(level.as_str(), "debug" | "info" | "warn" | "error") {
            return Err(ConfigError::Validation(format!(
                "log_level must be debug/info/warn/error, got {}",
                self.log_level
            )));
        }
        Ok(())
    }

    /// Returns true if `name` should be tracked under current whitelist rules.
    pub fn matches_device(&self, name: &str) -> bool {
        if !self.device_whitelist.is_empty() {
            return self
                .device_whitelist
                .iter()
                .any(|w| w.eq_ignore_ascii_case(name));
        }
        name.to_ascii_lowercase()
            .contains(&self.device_name.to_ascii_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn loads_valid_config() {
        let mut f = NamedTempFile::new().unwrap();
        write!(
            f,
            r#"
device_name: "Watch"
rssi_threshold: -75
grace_period: 5
ema_alpha: 0.3
scan_interval: 1
log_level: debug
auto_start: false
device_whitelist: ["Watch", "Phone"]
"#
        )
        .unwrap();
        let cfg = Config::load(f.path()).unwrap();
        assert_eq!(cfg.device_name, "Watch");
        assert_eq!(cfg.rssi_threshold, -75);
        assert_eq!(cfg.grace_period, 5);
        assert!((cfg.ema_alpha - 0.3).abs() < f64::EPSILON);
        assert!(!cfg.auto_start);
        assert_eq!(cfg.device_whitelist.len(), 2);
    }

    #[test]
    fn applies_defaults_for_optional_fields() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "device_name: \"iPhone\"\n").unwrap();
        let cfg = Config::load(f.path()).unwrap();
        assert_eq!(cfg.rssi_threshold, -80);
        assert_eq!(cfg.grace_period, 10);
        assert!((cfg.ema_alpha - 0.2).abs() < f64::EPSILON);
        assert_eq!(cfg.scan_interval, 2);
        assert_eq!(cfg.log_level, "info");
        assert!(cfg.auto_start);
        assert!(cfg.device_whitelist.is_empty());
    }

    #[test]
    fn rejects_invalid_ema_alpha() {
        let mut f = NamedTempFile::new().unwrap();
        write!(
            f,
            "device_name: \"x\"\nema_alpha: 1.5\n"
        )
        .unwrap();
        let err = Config::load(f.path()).unwrap_err();
        assert!(matches!(err, ConfigError::Validation(_)));
    }

    #[test]
    fn rejects_empty_device_name() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "device_name: \"\"\n").unwrap();
        assert!(Config::load(f.path()).is_err());
    }

    #[test]
    fn matches_device_with_whitelist() {
        let mut cfg = Config::default();
        cfg.device_whitelist = vec!["Galaxy Watch".into()];
        assert!(cfg.matches_device("Galaxy Watch"));
        assert!(!cfg.matches_device("iPhone"));
    }
}
