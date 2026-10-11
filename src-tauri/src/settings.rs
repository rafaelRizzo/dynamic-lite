//! Settings: fonte da verdade no Rust, JSON em Application Support, aplicadas na hora.

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt;

pub const CHANGED_EVENT: &str = "settings://changed";
const FILE: &str = "settings.json";

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    #[default]
    Black,
    Translucent,
    Glass,
    /// fundo na cor da capa da música
    Themed,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ExpandOn {
    #[default]
    Hover,
    Click,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Accent {
    #[default]
    Artwork,
    White,
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub style: Style,
    /// 0.6-1.0, só no Style Translucent
    pub opacity: f64,
    pub expand_on: ExpandOn,
    pub hover_delay_ms: u32,
    /// espera depois que o mouse sai antes de fechar; voltar nesse intervalo cancela
    pub hover_exit_delay_ms: u32,
    pub hide_idle_without_notch: bool,
    pub accent: Accent,
    pub haptics: bool,
    /// deslizar com 3 dedos na Island aberta troca de aba
    pub swipe_tabs: bool,
    /// inverte a direção do gesto
    pub invert_swipe: bool,
    pub show_menu_bar_icon: bool,
    /// "auto", "main" ou o nome da tela
    pub display: String,
    pub all_spaces: bool,
    pub hide_in_mission_control: bool,
    pub auto_update: bool,
    pub show_claude_usage: bool,
    /// consulta o limite direto na conta (token do Keychain) em vez de só ler o arquivo do statusline
    pub claude_live_limits: bool,
    pub show_agenda: bool,
    /// avisa N min antes de cada evento do Calendário; 0 desliga
    pub calendar_lead_min: u32,
    pub show_pomodoro: bool,
    /// minutos
    pub focus_min: u32,
    pub short_break_min: u32,
    pub long_break_min: u32,
    /// pausa longa a cada N focos
    pub long_every: u32,
    pub auto_start_next: bool,
    /// nome de um som de /System/Library/Sounds
    pub sfx: String,
    /// 0.1-1.0
    pub sfx_volume: f64,
    pub show_shortcuts: bool,
    /// ids do catálogo de atalhos do frontend, na ordem da grade
    pub shortcuts: Vec<String>,
}

const MAX_SHORTCUTS: usize = 12;

impl Default for Settings {
    fn default() -> Self {
        Self {
            style: Style::Black,
            opacity: 0.8,
            expand_on: ExpandOn::Hover,
            hover_delay_ms: 100,
            hover_exit_delay_ms: 300,
            hide_idle_without_notch: true,
            accent: Accent::Artwork,
            haptics: true,
            swipe_tabs: true,
            invert_swipe: false,
            show_menu_bar_icon: true,
            display: "auto".into(),
            all_spaces: true,
            hide_in_mission_control: true,
            auto_update: true,
            show_claude_usage: true,
            claude_live_limits: true,
            show_agenda: true,
            calendar_lead_min: 5,
            show_pomodoro: true,
            focus_min: 25,
            short_break_min: 5,
            long_break_min: 15,
            long_every: 4,
            auto_start_next: false,
            sfx: "Glass".into(),
            sfx_volume: 0.8,
            show_shortcuts: false,
            shortcuts: ["wifi", "bluetooth", "sound", "battery", "displays", "notifications"]
                .map(String::from)
                .to_vec(),
        }
    }
}

impl Settings {
    pub fn durations(&self) -> crate::pomodoro::Durations {
        let min = |m: u32| u64::from(m) * 60_000;
        crate::pomodoro::Durations {
            focus: min(self.focus_min),
            short: min(self.short_break_min),
            long: min(self.long_break_min),
            long_every: self.long_every,
        }
    }

    fn sanitized(mut self) -> Self {
        self.opacity = self.opacity.clamp(0.6, 1.0);
        self.hover_delay_ms = self.hover_delay_ms.min(1000);
        self.hover_exit_delay_ms = self.hover_exit_delay_ms.min(3000);
        if self.display.is_empty() {
            self.display = "auto".into();
        }
        self.calendar_lead_min = self.calendar_lead_min.min(60);
        self.focus_min = self.focus_min.clamp(1, 1440);
        self.short_break_min = self.short_break_min.clamp(1, 1440);
        self.long_break_min = self.long_break_min.clamp(1, 1440);
        self.long_every = self.long_every.clamp(2, 10);
        self.sfx_volume = self.sfx_volume.clamp(0.1, 1.0);
        if self.sfx.is_empty() || !self.sfx.chars().all(|c| c.is_ascii_alphanumeric()) {
            self.sfx = "Glass".into();
        }
        self.shortcuts.retain(|id| !id.is_empty() && id.len() <= 32);
        let mut seen = std::collections::HashSet::new();
        self.shortcuts.retain(|id| seen.insert(id.clone()));
        self.shortcuts.truncate(MAX_SHORTCUTS);
        self
    }
}

pub type SharedSettings = Arc<Mutex<Settings>>;

fn path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|dir| dir.join(FILE))
}

pub fn load(app: &AppHandle) -> Settings {
    path(app)
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|raw| serde_json::from_str::<Settings>(&raw).ok())
        .unwrap_or_default()
        .sanitized()
}

fn save(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let p = path(app).ok_or("pasta de configuração indisponível")?;
    if let Some(dir) = p.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(p, json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_settings(settings: State<'_, SharedSettings>) -> Settings {
    settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_settings(app: AppHandle, value: Settings, settings: State<'_, SharedSettings>) -> Result<(), String> {
    let value = value.sanitized();
    let previous = std::mem::replace(&mut *settings.lock().unwrap(), value.clone());
    save(&app, &value)?;

    if previous.display != value.display {
        crate::reposition(&app);
    }
    if (previous.all_spaces, previous.hide_in_mission_control) != (value.all_spaces, value.hide_in_mission_control) {
        crate::apply_spaces_behavior(&app);
    }
    if previous.show_menu_bar_icon != value.show_menu_bar_icon {
        crate::menu::set_tray_visible(&app, value.show_menu_bar_icon);
    }
    let _ = app.emit(CHANGED_EVENT, &value);
    Ok(())
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let launcher = app.autolaunch();
    if enabled { launcher.enable() } else { launcher.disable() }.map_err(|e| e.to_string())?;
    Ok(launcher.is_enabled().unwrap_or(enabled))
}
