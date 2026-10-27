//! Menu nativo (menu bar e Context Menu) e Settings Window.

use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindowBuilder, Window};

use crate::updater::Updates;

const TRAY_ID: &str = "tray";
const SETTINGS_LABEL: &str = "settings";

fn build_menu<R: Runtime, M: Manager<R>>(manager: &M) -> tauri::Result<Menu<R>> {
    let menu = Menu::new(manager)?;
    if let Some(update) = manager.state::<Updates>().info() {
        let label = format!("Instalar atualização v{} e reabrir", update.version);
        menu.append(&MenuItem::with_id(manager, "install-update", label, true, None::<&str>)?)?;
        menu.append(&PredefinedMenuItem::separator(manager)?)?;
    }
    menu.append(&MenuItem::with_id(manager, "settings", "Ajustes…", true, None::<&str>)?)?;
    menu.append(&PredefinedMenuItem::separator(manager)?)?;
    menu.append(&MenuItem::with_id(manager, "quit", "Sair do Dynamic Lite", true, None::<&str>)?)?;
    Ok(menu)
}

/// Remonta o menu da menu bar (ex.: apareceu uma atualização).
pub fn refresh_tray(app: &AppHandle) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY_ID), build_menu(app)) {
        let _ = tray.set_menu(Some(menu));
    }
}

pub fn setup_tray(app: &AppHandle, visible: bool) -> tauri::Result<()> {
    let tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().cloned().ok_or(tauri::Error::WindowNotFound)?)
        .tooltip("Dynamic Lite")
        .menu(&build_menu(app)?)
        .show_menu_on_left_click(true)
        .build(app)?;
    tray.set_visible(visible)?;
    Ok(())
}

pub fn set_tray_visible(app: &AppHandle, visible: bool) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_visible(visible);
    }
}

/// Handler global: recebe eventos do menu da menu bar e do Context Menu.
pub fn handle_event(app: &AppHandle, event: MenuEvent) {
    match event.id.as_ref() {
        "settings" => open_settings(app),
        "install-update" => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let _ = crate::updater::install(app).await;
            });
        }
        "quit" => app.exit(0),
        _ => {}
    }
}

pub fn open_settings(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(SETTINGS_LABEL) {
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let built = WebviewWindowBuilder::new(app, SETTINGS_LABEL, WebviewUrl::App("index.html".into()))
        .title("Ajustes do Dynamic Lite")
        .inner_size(500.0, 640.0)
        .resizable(false)
        .minimizable(false)
        .center()
        .build();
    if let Ok(win) = built {
        let _ = win.set_focus();
    }
}

#[tauri::command]
pub fn show_context_menu(window: Window) -> Result<(), String> {
    let menu = build_menu(&window).map_err(|e| e.to_string())?;
    window.popup_menu(&menu).map_err(|e| e.to_string())
}
