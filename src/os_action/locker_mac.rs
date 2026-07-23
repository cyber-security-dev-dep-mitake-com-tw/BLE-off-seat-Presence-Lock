use anyhow::{Context, Result, bail};
use std::process::Command;

use super::ScreenLocker;

/// macOS screen locker via CGSession -suspend.
pub struct MacLocker;

impl ScreenLocker for MacLocker {
    fn lock_screen(&self) -> Result<()> {
        // Legacy path used by older macOS releases.
        let legacy = "/System/Library/CoreServices/Menu Extras/User.menu/Contents/Resources/CGSession";
        if std::path::Path::new(legacy).exists() {
            let status = Command::new(legacy)
                .arg("-suspend")
                .status()
                .context("failed to invoke CGSession -suspend")?;
            if status.success() {
                return Ok(());
            }
            bail!("CGSession -suspend exited with {status}");
        }

        // Fallback for newer macOS where CGSession may be absent.
        let status = Command::new("osascript")
            .args([
                "-e",
                "tell application \"System Events\" to keystroke \"q\" using {command down, control down}",
            ])
            .status()
            .context("failed to invoke osascript lock shortcut")?;

        if status.success() {
            Ok(())
        } else {
            bail!("osascript lock exited with {status}")
        }
    }
}
