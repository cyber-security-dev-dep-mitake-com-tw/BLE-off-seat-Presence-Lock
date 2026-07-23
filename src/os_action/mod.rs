#[cfg(target_os = "macos")]
mod locker_mac;
#[cfg(target_os = "windows")]
mod locker_win;

use anyhow::Result;

/// Cross-platform screen lock abstraction.
pub trait ScreenLocker: Send + Sync {
    fn lock_screen(&self) -> Result<()>;
}

/// Logs instead of locking — used with `--mock` and unsupported platforms.
pub struct NoOpLocker;

impl ScreenLocker for NoOpLocker {
    fn lock_screen(&self) -> Result<()> {
        log::info!("[noop] screen lock requested (dry-run)");
        Ok(())
    }
}

/// Create the platform locker. When `dry_run` is true, never locks the OS.
pub fn create_locker(dry_run: bool) -> Box<dyn ScreenLocker> {
    if dry_run {
        return Box::new(NoOpLocker);
    }

    #[cfg(target_os = "macos")]
    {
        return Box::new(locker_mac::MacLocker);
    }

    #[cfg(target_os = "windows")]
    {
        return Box::new(locker_win::WinLocker);
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        log::warn!("screen lock unsupported on this platform; using no-op locker");
        Box::new(NoOpLocker)
    }
}
