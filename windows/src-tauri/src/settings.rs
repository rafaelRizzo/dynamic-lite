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

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum EqualizerStyle {
    #[default]
    Bars,
    /// GIF enviado pelo usuário (`equalizer_gif.rs`)
    Gif,
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub style: Style,
    /// 0.6-1.0, só no Style Translucent
    pub opacity: f64,
    /// tamanho da Island inteira, 0.7-1.6 (1 = padrão)
    pub island_scale: f64,
    pub expand_on: ExpandOn,
    pub hover_delay_ms: u32,
    /// espera depois que o mouse sai antes de fechar; voltar nesse intervalo cancela
    pub hover_exit_delay_ms: u32,
    pub hide_idle_without_notch: bool,
    pub accent: Accent,
    pub equalizer_style: EqualizerStyle,
    /// com o Spotify parado, o equalizer e a Island seguem o áudio de qualquer app (ADR 0010)
    pub system_audio: bool,
    /// lado do GIF em px
    pub equalizer_gif_size: u32,
    /// tamanho do GIF salvo (0 = nenhum). Só o Rust escreve, via `set_gif`
    pub equalizer_gif_bytes: u64,
    /// muda a cada GIF novo (ms desde a época), pro frontend recarregar mesmo com o mesmo tamanho
    pub equalizer_gif_rev: u64,
    pub haptics: bool,
    /// deslizar com 3 dedos na Island aberta troca de aba
    pub swipe_tabs: bool,
    /// inverte a direção do gesto
    pub invert_swipe: bool,
    /// reabre na última aba usada, inclusive depois de reiniciar o app
    pub remember_tab: bool,
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
    /// nome de um som da pasta Media do Windows
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
            island_scale: 1.0,
            expand_on: ExpandOn::Hover,
            hover_delay_ms: 100,
            hover_exit_delay_ms: 300,
            hide_idle_without_notch: true,
            accent: Accent::Artwork,
            equalizer_style: EqualizerStyle::Bars,
            system_audio: false,
            equalizer_gif_size: 24,
            equalizer_gif_bytes: 0,
            equalizer_gif_rev: 0,
            haptics: true,
            swipe_tabs: true,
            invert_swipe: false,
            remember_tab: false,
            show_menu_bar_icon: true,
            display: "auto".into(),
            all_spaces: true,
            hide_in_mission_control: true,
            auto_update: false,
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
            sfx: crate::sound::DEFAULT.into(),
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

    /// Registra (ou zera) o GIF salvo; sem GIF, o estilo volta pras barras.
    fn set_gif_bytes(&mut self, bytes: u64) {
        self.equalizer_gif_bytes = bytes;
        self.equalizer_gif_rev = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64);
        if bytes == 0 && self.equalizer_style == EqualizerStyle::Gif {
            self.equalizer_style = EqualizerStyle::Bars;
        }
    }

    fn sanitized(mut self) -> Self {
        self.opacity = self.opacity.clamp(0.6, 1.0);
        self.island_scale = if self.island_scale.is_finite() { self.island_scale.clamp(0.7, 1.6) } else { 1.0 };
        self.equalizer_gif_size = self.equalizer_gif_size.clamp(16, 32);
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
        if !crate::sound::valid_name(&self.sfx) {
            self.sfx = crate::sound::DEFAULT.into();
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
    let mut loaded = path(app)
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|raw| serde_json::from_str::<Settings>(raw.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_default()
        .sanitized();
    // o arquivo do GIF sumiu (apagado à mão): volta pras barras em vez de mostrar nada
    if loaded.equalizer_gif_bytes > 0 && !crate::equalizer_gif::exists(app) {
        loaded.set_gif_bytes(0);
    }
    loaded
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

/// Chamado por `equalizer_gif`: grava o tamanho do GIF e avisa as janelas.
pub fn set_gif(app: &AppHandle, shared: &SharedSettings, bytes: u64) -> Result<(), String> {
    let value = {
        let mut guard = shared.lock().unwrap();
        guard.set_gif_bytes(bytes);
        guard.clone()
    };
    save(app, &value)?;
    let _ = app.emit(CHANGED_EVENT, &value);
    Ok(())
}

#[tauri::command]
pub fn set_settings(app: AppHandle, value: Settings, settings: State<'_, SharedSettings>) -> Result<(), String> {
    let mut value = value.sanitized();
    {
        // dados do GIF são do Rust: o frontend não sobrescreve (pode estar com a cópia antiga)
        let current = settings.lock().unwrap();
        value.equalizer_gif_bytes = current.equalizer_gif_bytes;
        value.equalizer_gif_rev = current.equalizer_gif_rev;
    }
    if value.equalizer_gif_bytes == 0 && value.equalizer_style == EqualizerStyle::Gif {
        value.equalizer_style = EqualizerStyle::Bars;
    }
    let previous = std::mem::replace(&mut *settings.lock().unwrap(), value.clone());
    save(&app, &value)?;

    if previous.display != value.display || previous.island_scale != value.island_scale {
        crate::reposition(&app);
    }
    if previous.system_audio != value.system_audio {
        app.state::<crate::equalizer::Control>().refresh();
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
