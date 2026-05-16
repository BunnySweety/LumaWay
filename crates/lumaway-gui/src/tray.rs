use ksni::blocking::{Handle, TrayMethods};
use ksni::menu::StandardItem;
use ksni::{Category, Icon, MenuItem, OfflineReason, Status, ToolTip, Tray};
use std::sync::mpsc::Sender;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    Present,
    ToggleSync,
    Quit,
    WatcherOnline,
    WatcherOffline,
}

#[derive(Clone)]
pub struct TrayLabels {
    pub icon_name: String,
    pub show_window: String,
    pub start_sync: String,
    pub stop_sync: String,
    pub quit: String,
    pub ready: String,
    pub syncing: String,
}

#[derive(Default)]
pub struct TrayController {
    handle: Option<Handle<LumaWayTray>>,
}

impl TrayController {
    pub fn install(sender: Sender<TrayCommand>, labels: TrayLabels) -> Self {
        let tray = LumaWayTray {
            sender,
            labels,
            running: false,
        };
        match tray.assume_sni_available(true).spawn() {
            Ok(handle) => Self {
                handle: Some(handle),
            },
            Err(error) => {
                eprintln!("lumaway-gui: status notifier tray unavailable: {error}");
                Self::default()
            }
        }
    }

    pub fn set_running(&self, running: bool) {
        if let Some(handle) = &self.handle {
            let _ = handle.update(move |tray| {
                tray.running = running;
            });
        }
    }

    pub fn shutdown(&self) {
        if let Some(handle) = &self.handle {
            handle.shutdown().wait();
        }
    }
}

#[derive(Clone)]
struct LumaWayTray {
    sender: Sender<TrayCommand>,
    labels: TrayLabels,
    running: bool,
}

impl LumaWayTray {
    fn send(&self, command: TrayCommand) {
        let _ = self.sender.send(command);
    }

    fn sync_label(&self) -> String {
        if self.running {
            self.labels.stop_sync.clone()
        } else {
            self.labels.start_sync.clone()
        }
    }

    fn sync_icon(&self) -> String {
        if self.running {
            "media-playback-stop".into()
        } else {
            "media-playback-start".into()
        }
    }

    fn tooltip_description(&self) -> String {
        if self.running {
            self.labels.syncing.clone()
        } else {
            self.labels.ready.clone()
        }
    }
}

impl Tray for LumaWayTray {
    const MENU_ON_ACTIVATE: bool = true;

    fn id(&self) -> String {
        "lumaway".into()
    }

    fn category(&self) -> Category {
        Category::ApplicationStatus
    }

    fn title(&self) -> String {
        "LumaWay".into()
    }

    fn status(&self) -> Status {
        Status::Active
    }

    fn icon_name(&self) -> String {
        self.labels.icon_name.clone()
    }

    fn icon_pixmap(&self) -> Vec<Icon> {
        vec![lumaway_icon(32), lumaway_icon(16)]
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            icon_name: self.labels.icon_name.clone(),
            icon_pixmap: self.icon_pixmap(),
            title: self.title(),
            description: self.tooltip_description(),
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.send(TrayCommand::Present);
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: self.labels.show_window.clone(),
                icon_name: "window-new".into(),
                activate: Box::new(|tray: &mut Self| tray.send(TrayCommand::Present)),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: self.sync_label(),
                icon_name: self.sync_icon(),
                activate: Box::new(|tray: &mut Self| tray.send(TrayCommand::ToggleSync)),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: self.labels.quit.clone(),
                icon_name: "application-exit".into(),
                activate: Box::new(|tray: &mut Self| tray.send(TrayCommand::Quit)),
                ..Default::default()
            }
            .into(),
        ]
    }

    fn watcher_online(&self) {
        self.send(TrayCommand::WatcherOnline);
    }

    fn watcher_offline(&self, _reason: OfflineReason) -> bool {
        self.send(TrayCommand::WatcherOffline);
        true
    }
}

fn lumaway_icon(size: i32) -> Icon {
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    let center = (size - 1) as f32 / 2.0;
    let radius = size as f32 * 0.46;
    let stroke = (size as f32 * 0.18).max(2.0);
    let lower = size as f32 * 0.68;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let inside = dx * dx + dy * dy <= radius * radius;
            let draw_l = inside
                && ((x as f32) < center - stroke * 0.7 && (y as f32) > center - stroke * 1.4
                    || (y as f32) > lower && (x as f32) < center + stroke * 1.8);

            let (alpha, red, green, blue) = if draw_l {
                (255, 245, 252, 255)
            } else if inside {
                let t = y as f32 / (size - 1) as f32;
                let red = (18.0 + 30.0 * t) as u8;
                let green = (126.0 + 68.0 * (1.0 - t)) as u8;
                let blue = (220.0 + 24.0 * (1.0 - t)) as u8;
                (255, red, green, blue)
            } else {
                (0, 0, 0, 0)
            };

            data.extend_from_slice(&[alpha, red, green, blue]);
        }
    }

    Icon {
        width: size,
        height: size,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::{lumaway_icon, LumaWayTray, TrayCommand, TrayLabels};
    use ksni::{MenuItem, Tray};
    use std::sync::mpsc;

    fn labels() -> TrayLabels {
        TrayLabels {
            icon_name: "io.github.BunnySweety.LumaWay".into(),
            show_window: "Show LumaWay".into(),
            start_sync: "Start sync".into(),
            stop_sync: "Stop sync".into(),
            quit: "Quit".into(),
            ready: "Ready".into(),
            syncing: "Syncing".into(),
        }
    }

    fn tray(running: bool) -> (LumaWayTray, mpsc::Receiver<TrayCommand>) {
        let (sender, receiver) = mpsc::channel();
        (
            LumaWayTray {
                sender,
                labels: labels(),
                running,
            },
            receiver,
        )
    }

    #[test]
    fn tray_menu_switches_start_stop_label_with_state() {
        let (idle, _) = tray(false);
        let (running, _) = tray(true);

        assert_standard_label(&idle.menu()[1], "Start sync");
        assert_standard_label(&running.menu()[1], "Stop sync");
    }

    #[test]
    fn tray_activate_requests_window_presentation() {
        let (mut tray, receiver) = tray(false);

        tray.activate(0, 0);

        assert_eq!(receiver.try_recv().unwrap(), TrayCommand::Present);
    }

    #[test]
    fn tray_menu_toggle_requests_sync_toggle() {
        let (tray, receiver) = tray(false);
        let menu = tray.menu();
        let MenuItem::Standard(item) = &menu[1] else {
            panic!("sync menu item should be standard");
        };

        (item.activate)(&mut tray.clone());

        assert_eq!(receiver.try_recv().unwrap(), TrayCommand::ToggleSync);
    }

    #[test]
    fn tray_icon_pixmap_uses_argb32_size() {
        let icon = lumaway_icon(16);

        assert_eq!(icon.width, 16);
        assert_eq!(icon.height, 16);
        assert_eq!(icon.data.len(), 16 * 16 * 4);
        assert!(icon.data.chunks_exact(4).any(|pixel| pixel[0] == 255));
    }

    fn assert_standard_label(item: &MenuItem<LumaWayTray>, expected: &str) {
        let MenuItem::Standard(item) = item else {
            panic!("expected standard menu item");
        };
        assert_eq!(item.label, expected);
    }
}
