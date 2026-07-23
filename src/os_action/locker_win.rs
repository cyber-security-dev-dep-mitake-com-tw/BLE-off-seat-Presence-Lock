use anyhow::{Context, Result};
use windows::Win32::System::Shutdown::LockWorkStation;

use super::ScreenLocker;

/// Windows screen locker via user32 LockWorkStation.
pub struct WinLocker;

impl ScreenLocker for WinLocker {
    fn lock_screen(&self) -> Result<()> {
        // SAFETY: LockWorkStation has no parameters and is safe to call from
        // an interactive user session.
        unsafe { LockWorkStation() }.context("LockWorkStation failed")
    }
}
