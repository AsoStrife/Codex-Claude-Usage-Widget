use std::time::{Duration, Instant};

use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Position, Rect, Size, WebviewWindow,
};

use crate::state::{AppState, TrayAnchor};

const POPUP_LABEL: &str = "main";
const GAP_PX: i32 = 8;
/// Windows deactivates the window for a few frames when a drag begins; blur is
/// ignored for this long afterwards so the popup doesn't close mid-move.
const DRAG_BLUR_GRACE: Duration = Duration::from_millis(900);
/// A blur is only acted on if the window is *still* unfocused after this delay,
/// which filters out the transient deactivations Windows sends during a drag.
const BLUR_CONFIRM_DELAY: Duration = Duration::from_millis(160);

pub fn popup(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(POPUP_LABEL)
}

/// Shows/hides the popup when the tray icon is clicked, positioning it right
/// above (or, if there isn't room, below) the tray icon it was clicked from.
pub fn toggle_popup(app: &AppHandle, tray_rect: Rect) {
    let Some(window) = popup(app) else {
        return;
    };

    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
        return;
    }

    let anchor = TrayAnchor {
        position: to_physical_position(tray_rect.position),
        size: to_physical_size(tray_rect.size),
    };
    *app.state::<AppState>().tray_anchor.lock().unwrap() = Some(anchor);

    position_near_tray(&window, anchor);
    reveal(app, &window);
}

pub fn hide_popup(app: &AppHandle) {
    if let Some(window) = popup(app) {
        let _ = window.hide();
    }
}

/// Shows the popup without repositioning it (used by the tray menu's "Open"
/// item, which has no click coordinates to anchor to).
pub fn show_popup_default(app: &AppHandle) {
    let Some(window) = popup(app) else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.set_focus();
        return;
    }
    if let Some(anchor) = *app.state::<AppState>().tray_anchor.lock().unwrap() {
        position_near_tray(&window, anchor);
    }
    reveal(app, &window);
}

fn reveal(app: &AppHandle, window: &WebviewWindow) {
    let _ = window.show();
    let _ = window.set_focus();
    let _ = app.emit("popup://shown", ());
}

/// Suppresses "hide on blur" for a short while, so the deactivation Windows
/// sends at the start of a native drag doesn't close the popup.
pub fn suppress_blur(app: &AppHandle) {
    *app.state::<AppState>().suppress_blur_until.lock().unwrap() =
        Instant::now() + DRAG_BLUR_GRACE;
}

/// Handles a `Focused(false)` event: hides the popup only if the user really
/// clicked away, rather than on a transient deactivation caused by dragging.
pub fn on_blur(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.settings.lock().unwrap().hide_on_blur {
        return;
    }
    if Instant::now() < *state.suppress_blur_until.lock().unwrap() {
        return;
    }

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(BLUR_CONFIRM_DELAY).await;
        let state = app.state::<AppState>();
        if Instant::now() < *state.suppress_blur_until.lock().unwrap() {
            return;
        }
        if let Some(window) = popup(&app) {
            if !window.is_focused().unwrap_or(false) {
                let _ = window.hide();
            }
        }
    });
}

/// Resizes the popup to `logical_height` and re-anchors it to the tray icon,
/// keeping the edge nearest the taskbar pinned. Returns `false` when the window
/// is already that tall, so callers can avoid pointless native resizes.
pub fn resize_to_content(app: &AppHandle, logical_height: f64) -> Result<bool, String> {
    let window = popup(app).ok_or_else(|| "main window not found".to_string())?;

    // `inner_size`, not `outer_size`: `set_size` sets the *client* area, while
    // an undecorated window with a shadow reports a larger outer size (it keeps
    // a native frame that `WM_NCCALCSIZE` hides). Feeding the outer width back
    // in made the popup a little wider on every single resize.
    //
    // `logical_height` is measured by the frontend in CSS pixels, so the width
    // read back here (physical pixels) has to be converted to stay consistent
    // at 125/150/200% DPI.
    let scale_factor = window.scale_factor().unwrap_or(1.0).max(0.01);
    let current = window.inner_size().map_err(|err| err.to_string())?;
    let logical_width = current.width as f64 / scale_factor;
    let target = logical_height.clamp(120.0, 720.0);

    // Ignore sub-pixel churn: the frontend re-measures on every layout change
    // and a no-op resize here would otherwise bounce back as another layout.
    if ((current.height as f64 / scale_factor) - target).abs() < 1.0 {
        return Ok(false);
    }

    window
        .set_size(Size::Logical(tauri::LogicalSize::new(logical_width, target)))
        .map_err(|err| err.to_string())?;

    if let Some(anchor) = *app.state::<AppState>().tray_anchor.lock().unwrap() {
        position_near_tray(&window, anchor);
    }
    Ok(true)
}

fn position_near_tray(window: &WebviewWindow, anchor: TrayAnchor) {
    let Ok(size) = window.outer_size() else {
        return;
    };

    let tray_pos = anchor.position;
    let tray_size = anchor.size;

    let mut x = tray_pos.x as f64 + tray_size.width as f64 / 2.0 - size.width as f64 / 2.0;
    let mut y = tray_pos.y as f64 - size.height as f64 - GAP_PX as f64;

    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten());

    if let Some(monitor) = monitor {
        let m_pos = monitor.position();
        let m_size = monitor.size();
        let min_x = m_pos.x as f64;
        let max_x = (min_x + m_size.width as f64 - size.width as f64).max(min_x);
        x = x.clamp(min_x, max_x);

        // Not enough room above the tray icon (e.g. a top-mounted taskbar):
        // fall back to placing the popup below it instead.
        if y < m_pos.y as f64 {
            y = tray_pos.y as f64 + tray_size.height as f64 + GAP_PX as f64;
        }
        let min_y = m_pos.y as f64;
        let max_y = (min_y + m_size.height as f64 - size.height as f64).max(min_y);
        y = y.clamp(min_y, max_y);
    }

    let _ = window.set_position(Position::Physical(PhysicalPosition::new(x as i32, y as i32)));
}

fn to_physical_position(position: Position) -> PhysicalPosition<i32> {
    match position {
        Position::Physical(p) => p,
        Position::Logical(p) => PhysicalPosition::new(p.x as i32, p.y as i32),
    }
}

fn to_physical_size(size: Size) -> PhysicalSize<u32> {
    match size {
        Size::Physical(s) => s,
        Size::Logical(s) => PhysicalSize::new(s.width as u32, s.height as u32),
    }
}
