use anyhow::{Result, bail};
use windows::Win32::UI::WindowsAndMessaging::LockWorkStation;

use super::ScreenLocker;

/// Windows screen locker via user32 LockWorkStation.
pub struct WinLocker;

impl ScreenLocker for WinLocker {
    fn lock_screen(&self) -> Result<()> {
        // SAFETY: LockWorkStation has no parameters and is safe to call from
        // an interactive user session.
        let ok = unsafe { LockWorkStation() };
        if ok.as_bool() {
            Ok(())
        } else {
            bail!("LockWorkStation returned FALSE");
        }
    }
}
