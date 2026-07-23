use crate::filter::PresenceState;
use eframe::egui;
use std::sync::{Arc, Mutex};

/// Shared runtime status visible to the GUI and monitor loop.
#[derive(Debug, Clone)]
pub struct MonitorStatus {
    pub running: bool,
    pub device_name: String,
    pub raw_rssi: Option<i16>,
    pub smoothed_rssi: Option<f64>,
    pub state: PresenceState,
    pub last_message: String,
}

impl Default for MonitorStatus {
    fn default() -> Self {
        Self {
            running: false,
            device_name: String::new(),
            raw_rssi: None,
            smoothed_rssi: None,
            state: PresenceState::Present,
            last_message: "Idle".into(),
        }
    }
}

/// Commands the UI can send to the monitor loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiCommand {
    Start,
    Stop,
    Quit,
}

/// Minimal egui window for presence monitoring.
pub struct PresenceApp {
    pub status: Arc<Mutex<MonitorStatus>>,
    pub cmd_tx: crossbeam_channel::Sender<UiCommand>,
    pub auto_started: bool,
    tray: Option<crate::ui::tray::TrayHandle>,
}

impl PresenceApp {
    pub fn new(
        status: Arc<Mutex<MonitorStatus>>,
        cmd_tx: crossbeam_channel::Sender<UiCommand>,
        auto_start: bool,
    ) -> Self {
        let tray = crate::ui::tray::TrayHandle::try_create().ok();
        let mut app = Self {
            status,
            cmd_tx,
            auto_started: false,
            tray,
        };
        if auto_start {
            let _ = app.cmd_tx.send(UiCommand::Start);
            app.auto_started = true;
        }
        app
    }
}

impl eframe::App for PresenceApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(tray) = &self.tray {
            tray.poll(ctx, &self.cmd_tx);
        }

        let snapshot = self
            .status
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("BLE Presence Lock");
            ui.separator();

            ui.label(format!("Device: {}", snapshot.device_name));
            ui.label(format!(
                "Status: {}",
                if snapshot.running {
                    "Monitoring"
                } else {
                    "Stopped"
                }
            ));

            ui.label(format!(
                "Raw RSSI: {}",
                snapshot
                    .raw_rssi
                    .map(|v| format!("{v} dBm"))
                    .unwrap_or_else(|| "—".into())
            ));
            ui.label(format!(
                "Smoothed RSSI: {}",
                snapshot
                    .smoothed_rssi
                    .map(|v| format!("{v:.1} dBm"))
                    .unwrap_or_else(|| "—".into())
            ));

            let state_text = match snapshot.state {
                PresenceState::Present => "Present".to_string(),
                PresenceState::Departing { remaining_secs } => {
                    format!("Departing ({remaining_secs}s)")
                }
                PresenceState::Absent => "Absent (lock)".to_string(),
            };
            ui.label(format!("Presence: {state_text}"));
            ui.label(format!("Message: {}", snapshot.last_message));

            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button("Start").clicked() {
                    let _ = self.cmd_tx.send(UiCommand::Start);
                }
                if ui.button("Stop").clicked() {
                    let _ = self.cmd_tx.send(UiCommand::Stop);
                }
                if ui.button("Quit").clicked() {
                    let _ = self.cmd_tx.send(UiCommand::Quit);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
        });

        // Keep UI responsive while monitor updates status.
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}
