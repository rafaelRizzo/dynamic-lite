#[cfg(not(windows))]
compile_error!("Esta variante do Dynamic Lite só roda no Windows");

mod calendar;
mod claude_usage;
mod equalizer;
mod equalizer_gif;
mod gsmtc;
mod layout;
mod menu;
mod pomodoro;
mod scheduler;
mod settings;
mod sound;
mod spotify;
mod system_media;
mod updater;
mod volume;
mod win;

use std::os::windows::process::CommandExt;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::MacosLauncher;

use layout::{Layout, Notch};
use settings::SharedSettings;
use spotify::Spotify;

type SharedLayout = Arc<Mutex<Layout>>;

pub(crate) const MAIN_LABEL: &str = "main";
const HOVER_EVENT: &str = "island://hover";
const NOTCH_EVENT: &str = "island://notch";
const HOVER_POLL: Duration = Duration::from_millis(16);
/// Intervalo em que a posição da janela é reaplicada: o Windows pode tirá-la do lugar (barra de tarefas, DPI).
const TOPMOST_REFRESH: Duration = Duration::from_secs(2);
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Platform {
    os: &'static str,
    glass_supported: bool,
    calendar_supported: bool,
    haptics_supported: bool,
    spaces_supported: bool,
}

#[tauri::command]
fn get_notch(layout: State<'_, SharedLayout>) -> Notch {
    layout.lock().unwrap().notch
}

#[tauri::command]
fn set_island_size(width: f64, height: f64, layout: State<'_, SharedLayout>) {
    layout.lock().unwrap().island = (width, height);
}

/// No Windows os gestos do sistema não passam pelo app: não há o que capturar. Fica pela compatibilidade do frontend.
#[tauri::command]
fn set_swipe_capture(_on: bool) {}

#[tauri::command]
fn get_platform() -> Platform {
    Platform {
        os: "windows",
        glass_supported: false,
        calendar_supported: false,
        haptics_supported: false,
        spaces_supported: false,
    }
}

#[tauri::command]
fn list_displays(app: AppHandle) -> Vec<String> {
    win::screen_names(&app)
}

/// O Windows não tem o Liquid Glass: o frontend cai pro Translucent.
#[tauri::command]
fn set_glass() {}

#[tauri::command]
fn haptic() {}

/// Abre uma página dos Ajustes do Windows (`ms-settings:`); qualquer outra URL é recusada.
#[tauri::command]
fn open_system_settings(url: String) -> Result<(), String> {
    if !url.starts_with("ms-settings:") || url.chars().any(|c| c.is_whitespace() || c.is_control() || c == '&' || c == '"') {
        return Err("URL não permitida".into());
    }
    std::process::Command::new("cmd")
        .args(["/C", "start", "", &url])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Recalcula Target Screen/Notch e reposiciona a janela.
pub(crate) fn reposition(app: &AppHandle) {
    let (display, ui) = {
        let s = app.state::<SharedSettings>();
        let s = s.lock().unwrap();
        (s.display.clone(), s.island_scale)
    };
    let Some((screen, scale, notch)) = win::target_screen(app, &display) else { return };
    let Some(window) = app.get_webview_window(MAIN_LABEL) else { return };

    let layout = app.state::<SharedLayout>();
    let mut l = layout.lock().unwrap();
    // chamada periódica: só mexe na janela se a tela mudou de verdade
    let same = l.scale == scale
        && l.ui_scale == ui
        && (l.screen.x, l.screen.y, l.screen.w, l.screen.h) == (screen.x, screen.y, screen.w, screen.h);
    if !same {
        win::place_window(&window, screen, scale, ui);
    }
    l.ui_scale = ui;
    let changed = l.notch != notch;
    l.screen = screen;
    l.scale = scale;
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
/// Aproveita o laço pra reencaixar a janela se o Windows mudou a tela (resolução, monitor conectado).
fn spawn_hover_tracker(app: AppHandle, layout: SharedLayout) {
    thread::spawn(move || {
        let Some(win) = app.get_webview_window(MAIN_LABEL) else { return };
        let mut inside = false;
        let mut ticks = 0u32;
        let refresh_every = (TOPMOST_REFRESH.as_millis() / HOVER_POLL.as_millis()) as u32;
        loop {
            thread::sleep(HOVER_POLL);
            let (x, y) = win::mouse_location(&app);
            let now = layout.lock().unwrap().hit(x, y);
            if now != inside {
                inside = now;
                let _ = win.set_ignore_cursor_events(!inside);
                let _ = app.emit(HOVER_EVENT, inside);
            }
            ticks += 1;
            if ticks >= refresh_every {
                ticks = 0;
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || {
                    reposition(&handle);
                    win::refresh_window(&handle);
                });
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
        .plugin(tauri_plugin_dialog::init())
        .manage(updater::Updates::default())
        .manage(layout.clone())
        .manage(spotify.clone())
        .manage(system_media::Current::default())
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
            equalizer::equalizer_system_active,
            system_media::system_media,
            equalizer_gif::equalizer_gif_pick,
            equalizer_gif::equalizer_gif_remove,
            equalizer_gif::equalizer_gif_read,
            updater::get_update,
            updater::check_update,
            updater::install_update,
        ])
        .setup(move |app| {
            let loaded = settings::load(app.handle());
            let show_icon = loaded.show_menu_bar_icon;
            app.manage::<SharedSettings>(Arc::new(Mutex::new(loaded)));

            let window = app.get_webview_window(MAIN_LABEL).expect("janela main ausente");
            win::configure_window(&window);
            window.set_ignore_cursor_events(true)?;
            reposition(app.handle());
            window.show()?;

            menu::setup_tray(app.handle(), show_icon)?;
            spawn_hover_tracker(app.handle().clone(), layout.clone());
            let eq = equalizer::spawn(app.handle().clone());
            spotify::spawn_watcher(app.handle().clone(), spotify.clone(), wake_rx, eq);
            system_media::spawn(app.handle().clone(), app.state::<system_media::Current>().inner().clone());
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
