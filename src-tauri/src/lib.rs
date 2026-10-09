#[cfg(not(target_os = "macos"))]
compile_error!("Dynamic Lite só roda no macOS");

mod layout;
mod macos;
mod spotify;
mod tray;

use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use objc2::MainThreadMarker;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::MacosLauncher;

use layout::{Layout, Notch, WINDOW_SIZE};
use spotify::Spotify;

type SharedLayout = Arc<Mutex<Layout>>;

const HOVER_EVENT: &str = "island://hover";
const NOTCH_EVENT: &str = "island://notch";
const HOVER_POLL: Duration = Duration::from_millis(30);

#[tauri::command]
fn get_notch(layout: State<'_, SharedLayout>) -> Notch {
    layout.lock().unwrap().notch
}

#[tauri::command]
fn set_island_size(width: f64, height: f64, layout: State<'_, SharedLayout>) {
    layout.lock().unwrap().island = (width, height);
}

/// Recalcula tela/Notch e reposiciona a janela. Roda na main thread.
fn reposition(app: &AppHandle) {
    let Some(mtm) = MainThreadMarker::new() else { return };
    let Some((screen, notch)) = macos::primary_screen(mtm) else { return };
    let Some(win) = app.get_webview_window("main") else { return };
    let Ok(ns_window) = win.ns_window() else { return };

    macos::place_window(ns_window, screen, WINDOW_SIZE);
    let layout = app.state::<SharedLayout>();
    let mut l = layout.lock().unwrap();
    let changed = l.notch != notch;
    l.screen = screen;
    l.notch = notch;
    if l.island == (0.0, 0.0) {
        l.island = (notch.width, notch.height);
    }
    drop(l);
    if changed {
        let _ = app.emit(NOTCH_EVENT, notch);
    }
}

/// Click-through fora da Hit Region + eventos de hover pro frontend (ADR 0003).
fn spawn_hover_tracker(app: AppHandle, layout: SharedLayout) {
    thread::spawn(move || {
        let Some(win) = app.get_webview_window("main") else { return };
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
        .invoke_handler(tauri::generate_handler![
            get_notch,
            set_island_size,
            spotify::spotify_state,
            spotify::spotify_control,
            spotify::spotify_seek,
            spotify::spotify_volume,
        ])
        .setup(move |app| {
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let win = app.get_webview_window("main").expect("janela main ausente");
            macos::configure_window(win.ns_window()?);
            win.set_ignore_cursor_events(true)?;
            reposition(app.handle());
            win.show()?;

            let handle = app.handle().clone();
            macos::on_screen_change(move || reposition(&handle));

            let s = spotify.clone();
            macos::on_spotify_change(move || s.wake());

            tray::setup(app)?;
            spawn_hover_tracker(app.handle().clone(), layout.clone());
            spotify::spawn_watcher(app.handle().clone(), spotify.clone(), wake_rx);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o Dynamic Lite");
}
