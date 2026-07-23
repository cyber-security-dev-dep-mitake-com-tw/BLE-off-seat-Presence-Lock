use crate::ui::app::UiCommand;
use eframe::egui;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};

/// System tray icon + context menu for the GUI mode.
pub struct TrayHandle {
    _icon: TrayIcon,
    show_id: tray_icon::menu::MenuId,
    start_id: tray_icon::menu::MenuId,
    stop_id: tray_icon::menu::MenuId,
    quit_id: tray_icon::menu::MenuId,
}

impl TrayHandle {
    pub fn try_create() -> Result<Self, String> {
        let icon = make_icon().map_err(|e| e.to_string())?;

        let show = MenuItem::new("Show", true, None);
        let start = MenuItem::new("Start", true, None);
        let stop = MenuItem::new("Stop", true, None);
        let quit = MenuItem::new("Quit", true, None);

        let show_id = show.id().clone();
        let start_id = start.id().clone();
        let stop_id = stop.id().clone();
        let quit_id = quit.id().clone();

        let menu = Menu::new();
        menu.append(&show).map_err(|e| e.to_string())?;
        menu.append(&PredefinedMenuItem::separator())
            .map_err(|e| e.to_string())?;
        menu.append(&start).map_err(|e| e.to_string())?;
        menu.append(&stop).map_err(|e| e.to_string())?;
        menu.append(&PredefinedMenuItem::separator())
            .map_err(|e| e.to_string())?;
        menu.append(&quit).map_err(|e| e.to_string())?;

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("BLE Presence Lock")
            .with_icon(icon)
            .build()
            .map_err(|e| e.to_string())?;

        Ok(Self {
            _icon: tray,
            show_id,
            start_id,
            stop_id,
            quit_id,
        })
    }

    pub fn poll(&self, ctx: &egui::Context, cmd_tx: &crossbeam_channel::Sender<UiCommand>) {
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::DoubleClick { .. } = event {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.show_id {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            } else if event.id == self.start_id {
                let _ = cmd_tx.send(UiCommand::Start);
            } else if event.id == self.stop_id {
                let _ = cmd_tx.send(UiCommand::Stop);
            } else if event.id == self.quit_id {
                let _ = cmd_tx.send(UiCommand::Quit);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

fn make_icon() -> Result<Icon, tray_icon::Error> {
    // Simple 32x32 solid blue icon (RGBA).
    let size = 32u32;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let edge = x < 2 || y < 2 || x >= size - 2 || y >= size - 2;
            if edge {
                rgba.extend_from_slice(&[30, 64, 120, 255]);
            } else {
                rgba.extend_from_slice(&[70, 140, 220, 255]);
            }
        }
    }
    Icon::from_rgba(rgba, size, size)
}
