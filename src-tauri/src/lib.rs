#[cfg(not(target_os = "macos"))]
compile_error!("Dynamic Lite só roda no macOS");

mod layout;
mod macos;
mod menu;
mod settings;
mod spotify;

use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use objc2::MainThreadMarker;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::MacosLauncher;

use layout::{Layout, Notch, WINDOW_SIZE};
use settings::SharedSettings;
use spotify::Spotify;

type SharedLayout = Arc<Mutex<Layout>>;

const MAIN_LABEL: &str = "main";
const HOVER_EVENT: &str = "island://hover";
const NOTCH_EVENT: &str = "island://notch";
const HOVER_POLL: Duration = Duration::from_millis(30);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Platform {
    glass_supported: bool,
}

/// Roda `f` na main thread (AppKit exige) e devolve o resultado.
fn on_main<R: Send + 'static>(app: &AppHandle, f: impl FnOnce(MainThreadMarker) -> R + Send + 'static) -> Option<R> {
    if let Some(mtm) = MainThreadMarker::new() {
        return Some(f(mtm));
    }
    let (tx, rx) = mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(f(MainThreadMarker::new().expect("main thread")));
    })
    .ok()?;
    rx.recv().ok()
}

#[tauri::command]
fn get_notch(layout: State<'_, SharedLayout>) -> Notch {
    layout.lock().unwrap().notch
}

#[tauri::command]
fn set_island_size(width: f64, height: f64, layout: State<'_, SharedLayout>) {
    layout.lock().unwrap().island = (width, height);
}

#[tauri::command]
fn get_platform() -> Platform {
    Platform { glass_supported: macos::glass_supported() }
}

#[tauri::command]
fn list_displays(app: AppHandle) -> Vec<String> {
    on_main(&app, macos::screen_names).unwrap_or_default()
}

#[tauri::command]
fn set_glass(app: AppHandle, target: Option<macos::GlassShape>, rest: macos::GlassShape) {
    let Some(win) = app.get_webview_window(MAIN_LABEL) else { return };
    let Ok(ns_window) = win.ns_window() else { return };
    let ptr = ns_window as usize;
    on_main(&app, move |mtm| macos::set_glass(mtm, ptr as *mut std::ffi::c_void, target, rest));
}

#[tauri::command]
fn haptic(app: AppHandle) {
    on_main(&app, |_| macos::haptic());
}

/// Recalcula Target Screen/Notch e reposiciona a janela.
pub(crate) fn reposition(app: &AppHandle) {
    let display = app.state::<SharedSettings>().lock().unwrap().display.clone();
    let handle = app.clone();
    on_main(app, move |mtm| {
        let Some((screen, notch)) = macos::target_screen(mtm, &display) else { return };
        let Some(win) = handle.get_webview_window(MAIN_LABEL) else { return };
        let Ok(ns_window) = win.ns_window() else { return };
        macos::place_window(ns_window, screen, WINDOW_SIZE);

        let layout = handle.state::<SharedLayout>();
        let mut l = layout.lock().unwrap();
        let changed = l.notch != notch;
        l.screen = screen;
        l.notch = notch;
        if l.island == (0.0, 0.0) {
            l.island = (notch.width, notch.height);
        }
        drop(l);
        if changed {
            let _ = handle.emit(NOTCH_EVENT, notch);
        }
    });
}

/// Click-through fora da Hit Region + eventos de hover pro frontend (ADR 0003).
fn spawn_hover_tracker(app: AppHandle, layout: SharedLayout) {
    thread::spawn(move || {
        let Some(win) = app.get_webview_window(MAIN_LABEL) else { return };
        let mut inside = false;
        loop {
            thread::sleep(HOVER_POLL);
            let (x, y) = macos::mouse_location();
            let now = layout.lock().unwrap().hit(x, y);
            if now != inside {
                inside = now;
                let _ = win.set_ignore_cursor_events(!inside);
                let _ = app.emit(HOVER_EVENT, inside);
            }
        }
    });
}

pub fn run() {
    let layout: SharedLayout = Arc::default();
    let (wake_tx, wake_rx) = mpsc::channel();
    let spotify = Spotify::new(wake_tx);

    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .manage(layout.clone())
        .manage(spotify.clone())
        .on_menu_event(menu::handle_event)
        .invoke_handler(tauri::generate_handler![
            get_notch,
            set_island_size,
            get_platform,
            list_displays,
            set_glass,
            haptic,
            menu::show_context_menu,
            settings::get_settings,
            settings::set_settings,
            settings::get_autostart,
            settings::set_autostart,
            spotify::spotify_state,
            spotify::spotify_control,
            spotify::spotify_seek,
            spotify::spotify_volume,
            spotify::spotify_open,
        ])
        .setup(move |app| {
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let loaded = settings::load(app.handle());
            let show_icon = loaded.show_menu_bar_icon;
            app.manage::<SharedSettings>(Arc::new(Mutex::new(loaded)));

            let win = app.get_webview_window(MAIN_LABEL).expect("janela main ausente");
            macos::configure_window(win.ns_window()?);
            win.set_ignore_cursor_events(true)?;
            reposition(app.handle());
            win.show()?;

            let handle = app.handle().clone();
            macos::on_screen_change(move || reposition(&handle));

            let s = spotify.clone();
            macos::on_spotify_change(move || s.wake());

            menu::setup_tray(app.handle(), show_icon)?;
            spawn_hover_tracker(app.handle().clone(), layout.clone());
            spotify::spawn_watcher(app.handle().clone(), spotify.clone(), wake_rx);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o Dynamic Lite");
}
