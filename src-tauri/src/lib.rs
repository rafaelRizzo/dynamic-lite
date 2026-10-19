#[cfg(not(target_os = "macos"))]
compile_error!("Dynamic Lite só roda no macOS");

mod calendar;
mod claude_usage;
mod equalizer;
mod gesture_guard;
mod layout;
mod macos;
mod menu;
mod multitouch;
mod pomodoro;
mod scheduler;
mod settings;
mod sound;
mod spotify;
mod updater;

use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

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
const SWIPE_EVENT: &str = "island://swipe";
const HOVER_POLL: Duration = Duration::from_millis(16);
/// Intervalo mínimo entre dois gestos de troca de aba.
const SWIPE_COOLDOWN: Duration = Duration::from_millis(450);

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

/// A Island aberta com abas e o gesto de 3 dedos ativo: os gestos do sistema não passam enquanto o mouse está nela.
#[tauri::command]
fn set_swipe_capture(on: bool) {
    gesture_guard::set_capture(on);
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

/// Abre um painel dos Ajustes do Sistema; qualquer outra URL é recusada.
#[tauri::command]
fn open_system_settings(url: String) -> Result<(), String> {
    if !url.starts_with("x-apple.systempreferences:") || url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("URL não permitida".into());
    }
    std::process::Command::new("open").arg(&url).spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Gesto de 3 dedos pro lado: avisa o frontend se o mouse está na Island. `forward`: dedos pra esquerda.
fn handle_swipe(app: &AppHandle, layout: &SharedLayout, forward: bool) {
    static LAST: Mutex<Option<Instant>> = Mutex::new(None);
    let (enabled, invert) = {
        let s = app.state::<SharedSettings>();
        let s = s.lock().unwrap();
        (s.swipe_tabs, s.invert_swipe)
    };
    let (x, y) = macos::mouse_location();
    if !enabled || !layout.lock().unwrap().hit(x, y) {
        return;
    }
    // as duas fontes (toque bruto e swipe do sistema) podem disparar o mesmo gesto
    let mut last = LAST.lock().unwrap();
    if last.is_some_and(|t| t.elapsed() < SWIPE_COOLDOWN) {
        return;
    }
    *last = Some(Instant::now());
    let dir: i8 = if forward != invert { 1 } else { -1 };
    let _ = app.emit(SWIPE_EVENT, dir);
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

/// Aplica os Settings de mesas/Mission Control na janela da Island.
pub(crate) fn apply_spaces_behavior(app: &AppHandle) {
    let (all, hide) = {
        let s = app.state::<SharedSettings>();
        let s = s.lock().unwrap();
        (s.all_spaces, s.hide_in_mission_control)
    };
    let Some(win) = app.get_webview_window(MAIN_LABEL) else { return };
    let Ok(ns_window) = win.ns_window() else { return };
    let ptr = ns_window as usize;
    on_main(app, move |_| macos::set_spaces_behavior(ptr as *mut std::ffi::c_void, all, hide));
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
        // precisa ser o primeiro plugin: uma segunda instância fecha e a primeira abre os Ajustes
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || menu::open_settings(&handle));
        }))
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(updater::Updates::default())
        .manage(layout.clone())
        .manage(spotify.clone())
        .on_menu_event(menu::handle_event)
        .invoke_handler(tauri::generate_handler![
            get_notch,
            set_island_size,
            set_swipe_capture,
            get_platform,
            list_displays,
            set_glass,
            haptic,
            open_system_settings,
            menu::show_context_menu,
            settings::get_settings,
            settings::set_settings,
            scheduler::pomodoro_state,
            scheduler::pomodoro_start,
            scheduler::pomodoro_pause,
            scheduler::pomodoro_reset,
            scheduler::pomodoro_skip,
            scheduler::reminders_list,
            scheduler::reminders_add,
            scheduler::reminders_snooze,
            scheduler::reminders_done,
            claude_usage::claude_usage,
            calendar::calendar_access,
            calendar::calendar_request_access,
            calendar::calendar_events,
            sound::list_sounds,
            sound::play_sound,
            settings::get_autostart,
            settings::set_autostart,
            spotify::spotify_state,
            spotify::spotify_control,
            spotify::spotify_seek,
            spotify::spotify_volume,
            spotify::spotify_open,
            spotify::spotify_shuffle,
            spotify::spotify_repeat,
            equalizer::equalizer_fps,
            updater::get_update,
            updater::check_update,
            updater::install_update,
        ])
        .setup(move |app| {
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let loaded = settings::load(app.handle());
            let show_icon = loaded.show_menu_bar_icon;
            app.manage::<SharedSettings>(Arc::new(Mutex::new(loaded)));

            let win = app.get_webview_window(MAIN_LABEL).expect("janela main ausente");
            macos::configure_window(win.ns_window()?);
            apply_spaces_behavior(app.handle());
            win.set_ignore_cursor_events(true)?;
            reposition(app.handle());
            win.show()?;

            let handle = app.handle().clone();
            macos::on_screen_change(move || reposition(&handle));

            let s = spotify.clone();
            macos::on_spotify_change(move || s.wake());

            // 3 dedos pro lado com o mouse na Island: o frontend troca de aba se ela estiver aberta.
            // Toque bruto do trackpad (não depende dos gestos do sistema) e o evento de swipe do macOS como reserva.
            let swipe = {
                let (app, layout) = (app.handle().clone(), layout.clone());
                std::sync::Arc::new(move |forward: bool| handle_swipe(&app, &layout, forward))
            };
            let raw = swipe.clone();
            if !multitouch::start(move |left| raw(left)) {
                eprintln!("MultitouchSupport indisponível: só o gesto de swipe do sistema troca de aba");
            }
            {
                let (app, layout) = (app.handle().clone(), layout.clone());
                gesture_guard::start(
                    move || app.state::<SharedSettings>().lock().unwrap().swipe_tabs,
                    move || {
                        let (x, y) = macos::mouse_location();
                        layout.lock().unwrap().hit(x, y)
                    },
                );
            }
            macos::on_swipe(move |dx| {
                if dx != 0.0 {
                    swipe(dx > 0.0);
                }
            });

            menu::setup_tray(app.handle(), show_icon)?;
            spawn_hover_tracker(app.handle().clone(), layout.clone());
            let eq = equalizer::spawn(app.handle().clone());
            spotify::spawn_watcher(app.handle().clone(), spotify.clone(), wake_rx, eq);
            updater::spawn_checker(app.handle().clone());

            let (sched_tx, sched_rx) = mpsc::channel();
            let sched = scheduler::Scheduler::new(app.handle(), sched_tx);
            app.manage::<scheduler::SharedScheduler>(sched.clone());
            scheduler::spawn(app.handle().clone(), sched, sched_rx);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o Dynamic Lite");
}
