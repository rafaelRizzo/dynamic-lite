use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::App;
use tauri_plugin_autostart::ManagerExt;

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

pub fn setup(app: &App) -> tauri::Result<()> {
    let autostart_on = app.autolaunch().is_enabled().unwrap_or(false);
    let autostart = CheckMenuItem::with_id(app, "autostart", "Iniciar com o macOS", true, autostart_on, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair do Dynamic Lite", true, Some("Cmd+Q"))?;
    let menu = Menu::with_items(app, &[&autostart, &PredefinedMenuItem::separator(app)?, &quit])?;

    TrayIconBuilder::with_id("tray")
        .icon(pill_icon())
        .icon_as_template(true)
        .tooltip("Dynamic Lite")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "autostart" => {
                let launcher = app.autolaunch();
                let enable = !launcher.is_enabled().unwrap_or(false);
                let _ = if enable { launcher.enable() } else { launcher.disable() };
                let _ = autostart.set_checked(launcher.is_enabled().unwrap_or(enable));
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
