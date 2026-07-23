mod ble;
mod config;
mod filter;
mod os_action;
mod ui;

use crate::ble::Scanner;
use crate::config::Config;
use crate::filter::{Debouncer, EmaFilter, PresenceState};
use crate::os_action::{ScreenLocker, create_locker};
use crate::ui::app::{MonitorStatus, PresenceApp, UiCommand};
use anyhow::{Context, Result};
use clap::Parser;
use egui::ViewportBuilder;
use log::{error, info, warn};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Parser)]
#[command(
    name = "ble-presence-lock",
    version,
    about = "Cross-platform BLE presence detection with auto screen lock"
)]
struct Cli {
    /// Run without GUI (daemon mode)
    #[arg(long)]
    headless: bool,

    /// Path to config.yaml (default: auto-detect beside binary / app support)
    #[arg(long)]
    config: Option<PathBuf>,

    /// Use mock BLE scanner (for testing)
    #[arg(long)]
    mock: bool,
}

fn main() {
    if let Err(err) = try_main() {
        eprintln!("Error: {err:?}");
        show_startup_error(&format!("{err:#}"));
        std::process::exit(1);
    }
}

fn try_main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = cli
        .config
        .clone()
        .unwrap_or_else(Config::resolve_default_path);

    let config = Config::load(&config_path).with_context(|| {
        format!(
            "failed to load config from {} (copy config.example.yaml or pass --config)",
            config_path.display()
        )
    })?;

    init_logger(&config.log_level);

    info!(
        "BLE Presence Lock v{} starting (headless={}, mock={}, config={})",
        env!("CARGO_PKG_VERSION"),
        cli.headless,
        cli.mock,
        config_path.display()
    );
    info!(
        "tracking device='{}' threshold={} dBm grace={}s",
        config.device_name, config.rssi_threshold, config.grace_period
    );

    // Mock mode never locks the real workstation.
    let locker = create_locker(cli.mock);

    if cli.headless {
        let rt = tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
        rt.block_on(run_headless(config, cli.mock, locker))
    } else {
        run_gui(config, cli.mock, locker)
    }
}

/// Show a visible error when launched from Finder (stderr alone is invisible).
fn show_startup_error(message: &str) {
    #[cfg(target_os = "macos")]
    {
        let escaped = message.replace('\\', "\\\\").replace('"', "\\\"");
        let script = format!(
            "display dialog \"{escaped}\" with title \"BLE Presence Lock\" buttons {{\"OK\"}} default button 1 with icon stop"
        );
        let _ = std::process::Command::new("osascript")
            .args(["-e", &script])
            .status();
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let escaped = message.replace('\'', "''");
        let _ = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Add-Type -AssemblyName PresentationFramework; [System.Windows.MessageBox]::Show('{escaped}','BLE Presence Lock','OK','Error') | Out-Null"
                ),
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }
}

fn init_logger(level: &str) {
    let level = match level.to_ascii_lowercase().as_str() {
        "debug" => "debug",
        "warn" => "warn",
        "error" => "error",
        _ => "info",
    };
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| {
        format!("ble_presence_lock={level},btleplug={level}")
    });
    let _ = env_logger::Builder::new().parse_filters(&filter).try_init();
}

async fn run_headless(
    config: Config,
    use_mock: bool,
    locker: Box<dyn ScreenLocker>,
) -> Result<()> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = stop.clone();

    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            info!("received Ctrl-C, shutting down");
            stop_flag.store(true, Ordering::SeqCst);
        }
    });

    let mut scanner = if use_mock {
        info!("using mock BLE scanner");
        Scanner::mock(config.debug_mock_file.clone())
    } else {
        Scanner::real().await?
    };

    let status = Arc::new(Mutex::new(MonitorStatus {
        device_name: config.device_name.clone(),
        running: true,
        last_message: "Headless monitoring".into(),
        ..MonitorStatus::default()
    }));

    let mut engine = PresenceEngine::new(&config);

    info!("headless monitor started");
    while !stop.load(Ordering::SeqCst) {
        if let Err(e) = engine
            .tick(&config, &mut scanner, locker.as_ref(), &status)
            .await
        {
            warn!("monitor tick error: {e}");
        }
        sleep(Duration::from_secs(config.scan_interval.max(1))).await;
    }
    info!("headless monitor stopped");
    Ok(())
}

fn run_gui(config: Config, use_mock: bool, locker: Box<dyn ScreenLocker>) -> Result<()> {
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<UiCommand>();
    let status = Arc::new(Mutex::new(MonitorStatus {
        device_name: config.device_name.clone(),
        running: false,
        ..MonitorStatus::default()
    }));

    let status_for_monitor = status.clone();
    let config_for_monitor = config.clone();
    let mock_file = config.debug_mock_file.clone();

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                error!("failed to create tokio runtime: {e}");
                return;
            }
        };

        rt.block_on(async move {
            let mut scanner = if use_mock {
                Scanner::mock(mock_file)
            } else {
                match Scanner::real().await {
                    Ok(s) => s,
                    Err(e) => {
                        if let Ok(mut s) = status_for_monitor.lock() {
                            s.last_message = format!("BLE init failed: {e}");
                        }
                        error!("BLE init failed: {e}");
                        return;
                    }
                }
            };

            let mut engine = PresenceEngine::new(&config_for_monitor);
            let mut active = config_for_monitor.auto_start;
            if let Ok(mut s) = status_for_monitor.lock() {
                s.running = active;
                if active {
                    s.last_message = "Monitoring started".into();
                }
            }

            loop {
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        UiCommand::Start => {
                            active = true;
                            engine.reset();
                            if let Ok(mut s) = status_for_monitor.lock() {
                                s.running = true;
                                s.last_message = "Monitoring started".into();
                            }
                        }
                        UiCommand::Stop => {
                            active = false;
                            if let Ok(mut s) = status_for_monitor.lock() {
                                s.running = false;
                                s.last_message = "Monitoring stopped".into();
                            }
                        }
                        UiCommand::Quit => return,
                    }
                }

                if active {
                    if let Err(e) = engine
                        .tick(
                            &config_for_monitor,
                            &mut scanner,
                            locker.as_ref(),
                            &status_for_monitor,
                        )
                        .await
                    {
                        warn!("monitor tick error: {e}");
                        if let Ok(mut s) = status_for_monitor.lock() {
                            s.last_message = format!("Error: {e}");
                        }
                    }
                }

                sleep(Duration::from_secs(config_for_monitor.scan_interval.max(1))).await;
            }
        });
    });

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([460.0, 360.0])
            .with_title("BLE Presence Lock"),
        ..Default::default()
    };

    eframe::run_native(
        "BLE Presence Lock",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(PresenceApp::new(
                status,
                cmd_tx,
                config.auto_start,
            )))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    Ok(())
}

/// Owns EMA + grace-period state across scan ticks.
struct PresenceEngine {
    ema: EmaFilter,
    debouncer: Debouncer,
    already_locked: bool,
    /// Do not lock until the target has been observed at least once.
    armed: bool,
    threshold: i16,
}

impl PresenceEngine {
    fn new(config: &Config) -> Self {
        Self {
            ema: EmaFilter::new(config.ema_alpha),
            debouncer: Debouncer::new(config.rssi_threshold, config.grace_period),
            already_locked: false,
            armed: false,
            threshold: config.rssi_threshold,
        }
    }

    fn reset(&mut self) {
        self.ema.reset();
        self.debouncer.reset();
        self.already_locked = false;
        self.armed = false;
    }

    async fn tick(
        &mut self,
        config: &Config,
        scanner: &mut Scanner,
        locker: &dyn ScreenLocker,
        status: &Arc<Mutex<MonitorStatus>>,
    ) -> Result<()> {
        let cycle = scanner
            .scan_once(&config.device_name, &config.device_whitelist)
            .await?;

        set_candidates(status, cycle.candidates.clone());

        match cycle.reading {
            Some(reading) => {
                self.armed = true;
                let smoothed = self.ema.update(reading.rssi as f64);
                let state = self.debouncer.update(smoothed);
                update_status(
                    status,
                    Some(reading.rssi),
                    Some(smoothed),
                    state,
                    Some(format!(
                        "Seen via {} as '{}'",
                        cycle.source, reading.device_name
                    )),
                    true,
                );

                info!(
                    "device='{}' raw={} smoothed={:.1} state={:?} source={}",
                    reading.device_name, reading.rssi, smoothed, state, cycle.source
                );

                self.handle_state(state, locker, status, Some(reading.rssi), smoothed)?;
            }
            None => {
                let hint = if cycle.candidates.is_empty() {
                    format!(
                        "Target '{}' not seen. No paired/BLE names found — check Bluetooth permission and System Settings → Bluetooth pairing.",
                        config.device_name
                    )
                } else {
                    format!(
                        "Target '{}' not seen. Set device_name in config.yaml to one of: {}",
                        config.device_name,
                        cycle.candidates.iter().take(8).cloned().collect::<Vec<_>>().join(", ")
                    )
                };

                if !self.armed {
                    // Misconfiguration / not yet paired: do NOT lock.
                    update_status(
                        status,
                        None,
                        None,
                        PresenceState::Present,
                        Some(hint),
                        false,
                    );
                    warn!("target not seen yet; lock armed=false");
                    return Ok(());
                }

                // Armed: device previously seen, now missing ⇒ treat as away.
                let synthetic = f64::from(self.threshold) - 20.0;
                let smoothed = self.ema.update(synthetic);
                let state = self.debouncer.update(smoothed);
                update_status(
                    status,
                    None,
                    Some(smoothed),
                    state,
                    Some(hint),
                    true,
                );
                warn!("target device lost; treating as away");
                self.handle_state(state, locker, status, None, smoothed)?;
            }
        }
        Ok(())
    }

    fn handle_state(
        &mut self,
        state: PresenceState,
        locker: &dyn ScreenLocker,
        status: &Arc<Mutex<MonitorStatus>>,
        raw: Option<i16>,
        smoothed: f64,
    ) -> Result<()> {
        if state == PresenceState::Absent {
            if !self.already_locked {
                info!("grace period elapsed — locking screen");
                match locker.lock_screen() {
                    Ok(()) => {
                        self.already_locked = true;
                        update_status(
                            status,
                            raw,
                            Some(smoothed),
                            state,
                            Some("Screen locked".into()),
                            true,
                        );
                    }
                    Err(e) => {
                        error!("lock failed: {e}");
                        update_status(
                            status,
                            raw,
                            Some(smoothed),
                            state,
                            Some(format!("Lock failed: {e}")),
                            true,
                        );
                        return Err(e);
                    }
                }
            }
        } else {
            self.already_locked = false;
        }
        Ok(())
    }
}

fn set_candidates(status: &Arc<Mutex<MonitorStatus>>, candidates: Vec<String>) {
    if let Ok(mut s) = status.lock() {
        s.candidates = candidates;
    }
}

fn update_status(
    status: &Arc<Mutex<MonitorStatus>>,
    raw: Option<i16>,
    smoothed: Option<f64>,
    state: PresenceState,
    message: Option<String>,
    armed: bool,
) {
    if let Ok(mut s) = status.lock() {
        s.raw_rssi = raw;
        s.smoothed_rssi = smoothed;
        s.state = state;
        s.armed = armed;
        if let Some(m) = message {
            s.last_message = m;
        }
    }
}
