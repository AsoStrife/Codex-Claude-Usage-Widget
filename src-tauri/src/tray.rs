use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuEvent};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::AppHandle;

use crate::window;

const TRAY_ICON_BYTES: &[u8] = include_bytes!("../icons/32x32.png");

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let icon = Image::from_bytes(TRAY_ICON_BYTES)?;

    let menu = MenuBuilder::new(app)
        .text("open", "Open")
        .text("refresh", "Refresh")
        .separator()
        .text("autostart", "Start with Windows")
        .separator()
        .text("quit", "Quit")
        .build()?;

    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("AI Usage")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                window::toggle_popup(tray.app_handle(), rect);
            }
        })
        .build(app)?;

    Ok(())
}

fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        "open" => window::show_popup_default(app),
        "refresh" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                crate::commands::refresh_all_and_emit(&app).await;
            });
        }
        "autostart" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let _ = crate::commands::toggle_start_with_windows(&app).await;
            });
        }
        "quit" => app.exit(0),
        _ => {}
    }
}
