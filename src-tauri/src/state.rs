use std::sync::Mutex;
use std::time::Instant;

use tauri::{PhysicalPosition, PhysicalSize};

use crate::providers::codex::CodexState;
use crate::types::{AppSettings, UsageSnapshot};

/// Where the tray icon was when the popup was last opened from it. The popup
/// is anchored to this rect, so it has to be re-applied every time the window
/// is resized to fit its content (otherwise growing the window pushes it down
/// behind the taskbar).
#[derive(Clone, Copy)]
pub struct TrayAnchor {
    pub position: PhysicalPosition<i32>,
    pub size: PhysicalSize<u32>,
}

pub struct AppState {
    pub settings: Mutex<AppSettings>,
    pub usage: Mutex<UsageSnapshot>,
    pub codex: CodexState,
    pub tray_anchor: Mutex<Option<TrayAnchor>>,
    /// Blur is ignored until this instant. Windows briefly deactivates the
    /// window when a drag starts (`ReleaseCapture` + `WM_NCLBUTTONDOWN`), and
    /// without this guard "hide on blur" would close the popup the moment the
    /// user tries to move it.
    pub suppress_blur_until: Mutex<Instant>,
}

impl AppState {
    pub fn new(settings: AppSettings) -> Self {
        Self {
            settings: Mutex::new(settings),
            usage: Mutex::new(UsageSnapshot::default()),
            codex: CodexState::default(),
            tray_anchor: Mutex::new(None),
            suppress_blur_until: Mutex::new(Instant::now()),
        }
    }
}
