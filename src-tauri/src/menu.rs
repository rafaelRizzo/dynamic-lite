//! Menu nativo (menu bar e Context Menu) e Settings Window.

use tauri::image::Image;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindowBuilder, Window};

const TRAY_ID: &str = "tray";
const SETTINGS_LABEL: &str = "settings";
const ICON_W: u32 = 36;
const ICON_H: u32 = 22;

/// Ícone template (pílula) desenhado em memória: o macOS pinta conforme o tema da menu bar.
fn pill_icon() -> Image<'static> {
    let (w, h) = (ICON_W as f64, ICON_H as f64);
    let (pw, ph) = (30.0, 12.0);
    let (x0, y0) = ((w - pw) / 2.0, (h - ph) / 2.0);
    let r = ph / 2.0;
    let mut rgba = vec![0u8; (ICON_W * ICON_H * 4) as usize];
    for y in 0..ICON_H {
        for x in 0..ICON_W {
            // distância até o retângulo arredondado, com antialias de 1px
            let px = x as f64 + 0.5;
            let py = y as f64 + 0.5;
            let cx = px.clamp(x0 + r, x0 + pw - r);
            let cy = py.clamp(y0 + r, y0 + ph - r);
            let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt() - r;
            let a = (0.5 - d).clamp(0.0, 1.0);
            let i = ((y * ICON_W + x) * 4) as usize;
            rgba[i + 3] = (a * 255.0) as u8;
        }
    }
    Image::new_owned(rgba, ICON_W, ICON_H)
}

fn build_menu<R: Runtime, M: Manager<R>>(manager: &M) -> tauri::Result<Menu<R>> {
    let settings = MenuItem::with_id(manager, "settings", "Ajustes…", true, Some("Cmd+,"))?;
    let quit = MenuItem::with_id(manager, "quit", "Sair do Dynamic Lite", true, Some("Cmd+Q"))?;
    Menu::with_items(manager, &[&settings, &PredefinedMenuItem::separator(manager)?, &quit])
}

pub fn setup_tray(app: &AppHandle, visible: bool) -> tauri::Result<()> {
    let tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(pill_icon())
        .icon_as_template(true)
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
        "quit" => app.exit(0),
        _ => {}
    }
}

pub fn open_settings(app: &AppHandle) {
    crate::macos::activate_app();
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
