//! Now Playing do Spotify via AppleScript (ADR 0002).

use std::process::Command;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::macos;

const BUNDLE_ID: &str = "com.spotify.client";
pub const STATE_EVENT: &str = "spotify://state";

const READ_SCRIPT: &str = r#"
tell application "Spotify"
  set s to player state as string
  if s is "stopped" then return "stopped"
  set t to current track
  set n to linefeed
  return s & n & (id of t) & n & (name of t) & n & (artist of t) & n & (album of t) & n & (artwork url of t) & n & ((duration of t) as integer) & n & ((player position * 1000) as integer) & n & sound volume & n & shuffling & n & repeating
end tell
"#;

#[derive(Clone, Copy, Default, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Closed,
    Stopped,
    Playing,
    Paused,
}

/// `Track` não existe no AppleScript do Spotify (só repeat liga/desliga): é emulado no watcher.
#[derive(Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Repeat {
    #[default]
    Off,
    Context,
    Track,
}

#[derive(Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub name: String,
    pub artist: String,
    pub album: String,
    pub artwork_url: String,
    /// segundos
    pub duration: f64,
}

#[derive(Clone, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub status: Status,
    pub track: Option<Track>,
    /// segundos, no momento da leitura
    pub position: f64,
    /// 0-100
    pub volume: u8,
    pub shuffle: bool,
    pub repeat: Repeat,
}

pub struct Spotify {
    last: Mutex<PlayerState>,
    waker: Mutex<Sender<()>>,
    /// Faixa em "repetir faixa" (emulado), se ativo.
    repeat_track: Mutex<Option<String>>,
}

impl Spotify {
    pub fn new(waker: Sender<()>) -> Arc<Self> {
        Arc::new(Self { last: Mutex::default(), waker: Mutex::new(waker), repeat_track: Mutex::default() })
    }

    fn repeat_track(&self) -> Option<String> {
        self.repeat_track.lock().unwrap().clone()
    }

    fn set_repeat_track(&self, id: Option<String>) {
        *self.repeat_track.lock().unwrap() = id;
    }

    /// Força uma leitura imediata no watcher.
    pub fn wake(&self) {
        let _ = self.waker.lock().unwrap().send(());
    }
}

fn osascript(script: &str) -> Result<String, String> {
    let out = Command::new("osascript")
        .args(["-e", script])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_owned())
    }
}

fn parse(raw: &str) -> PlayerState {
    let f: Vec<&str> = raw.split('\n').collect();
    let status = match f.first().copied() {
        Some("playing") => Status::Playing,
        Some("paused") => Status::Paused,
        _ => return PlayerState { status: Status::Stopped, ..Default::default() },
    };
    if f.len() < 11 {
        return PlayerState { status: Status::Stopped, ..Default::default() };
    }
    let ms = |s: &str| s.trim().parse::<f64>().unwrap_or(0.0) / 1000.0;
    PlayerState {
        status,
        track: Some(Track {
            id: f[1].into(),
            name: f[2].into(),
            artist: f[3].into(),
            album: f[4].into(),
            artwork_url: f[5].into(),
            duration: ms(f[6]),
        }),
        position: ms(f[7]),
        volume: f[8].trim().parse().unwrap_or(0),
        shuffle: f[9].trim() == "true",
        repeat: if f[10].trim() == "true" { Repeat::Context } else { Repeat::Off },
    }
}

fn read_state() -> PlayerState {
    // checa antes: `tell application` abriria o Spotify se estivesse fechado
    if !macos::is_app_running(BUNDLE_ID) {
        return PlayerState::default();
    }
    match osascript(READ_SCRIPT) {
        Ok(raw) => parse(&raw),
        Err(_) => PlayerState { status: Status::Stopped, ..Default::default() },
    }
}

/// Quanto antes do fim da faixa o "repetir faixa" volta pro começo.
const REPEAT_ONE_LEAD: f64 = 0.8;
/// Faixa trocou com a anterior a menos disso do fim: foi fim natural, não pulo manual.
const NATURAL_END_WINDOW: f64 = 5.0;

/// Emula "repetir faixa": volta pro começo perto do fim; se o Spotify avançou mesmo assim, volta pra faixa;
/// se o usuário pulou de faixa, vira "repetir tudo" (como o próprio Spotify faz).
fn repeat_one(spotify: &Spotify, id: &str, state: PlayerState, prev: Option<&(PlayerState, Instant)>) -> PlayerState {
    let Some(track) = &state.track else { return state };
    if track.id == id {
        if state.status == Status::Playing && track.duration - state.position <= REPEAT_ONE_LEAD {
            let _ = osascript(r#"tell application "Spotify" to set player position to 0"#);
            return read_state();
        }
        return state;
    }
    let natural_end = prev.is_some_and(|(p, at)| {
        p.track.as_ref().is_some_and(|t| {
            let elapsed = if p.status == Status::Playing { at.elapsed().as_secs_f64() } else { 0.0 };
            t.id == id && t.duration - (p.position + elapsed) < NATURAL_END_WINDOW
        })
    });
    if natural_end {
        // no começo da faixa nova, "previous track" volta pra anterior (a repetida)
        let _ = osascript(r#"tell application "Spotify" to previous track"#);
        thread::sleep(Duration::from_millis(150));
        read_state()
    } else {
        spotify.set_repeat_track(None);
        state
    }
}

/// Lê o estado quando acordado (notificação do Spotify ou comando) ou num polling lento de segurança.
pub fn spawn_watcher(app: AppHandle, spotify: Arc<Spotify>, rx: Receiver<()>) {
    thread::spawn(move || {
        let mut prev: Option<(PlayerState, Instant)> = None;
        loop {
            let mut state = read_state();
            let repeating_one = spotify.repeat_track();
            if let Some(id) = &repeating_one {
                state = repeat_one(&spotify, id, state, prev.as_ref());
            }
            if spotify.repeat_track().is_some() {
                state.repeat = Repeat::Track;
            }

            let mut interval = match state.status {
                Status::Playing => Duration::from_secs(3),
                Status::Closed => Duration::from_secs(5),
                _ => Duration::from_secs(4),
            };
            // repetindo faixa: acorda a tempo de voltar pro começo antes do fim
            if let (Repeat::Track, Status::Playing, Some(t)) = (state.repeat, state.status, &state.track) {
                let until_lead = (t.duration - state.position - REPEAT_ONE_LEAD).max(0.15);
                interval = interval.min(Duration::from_secs_f64(until_lead));
            }

            {
                let mut last = spotify.last.lock().unwrap();
                if *last != state {
                    let _ = app.emit(STATE_EVENT, &state);
                    *last = state.clone();
                }
            }
            prev = Some((state, Instant::now()));

            match rx.recv_timeout(interval) {
                Ok(()) => {
                    // o Spotify às vezes ainda não atualizou a posição no instante da notificação
                    thread::sleep(Duration::from_millis(120));
                    while rx.try_recv().is_ok() {}
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    });
}

async fn run(script: String) -> Result<(), String> {
    if !macos::is_app_running(BUNDLE_ID) {
        return Ok(());
    }
    tauri::async_runtime::spawn_blocking(move || osascript(&script))
        .await
        .map_err(|e| e.to_string())?
        .map(|_| ())
}

#[tauri::command]
pub fn spotify_state(spotify: State<'_, Arc<Spotify>>) -> PlayerState {
    spotify.last.lock().unwrap().clone()
}

#[tauri::command]
pub async fn spotify_control(action: String, spotify: State<'_, Arc<Spotify>>) -> Result<(), String> {
    let verb = match action.as_str() {
        "playpause" => "playpause",
        "next" => "next track",
        "previous" => "previous track",
        other => return Err(format!("ação desconhecida: {other}")),
    };
    // pular de faixa sai do "repetir faixa" (vira "repetir tudo"), como no Spotify
    if action != "playpause" {
        spotify.set_repeat_track(None);
    }
    run(format!(r#"tell application "Spotify" to {verb}"#)).await?;
    spotify.wake();
    Ok(())
}

#[tauri::command]
pub async fn spotify_seek(position: f64, spotify: State<'_, Arc<Spotify>>) -> Result<(), String> {
    let pos = position.max(0.0);
    run(format!(r#"tell application "Spotify" to set player position to {pos:.3}"#)).await?;
    spotify.wake();
    Ok(())
}

#[tauri::command]
pub async fn spotify_volume(volume: u8, spotify: State<'_, Arc<Spotify>>) -> Result<(), String> {
    let v = volume.min(100);
    run(format!(r#"tell application "Spotify" to set sound volume to {v}"#)).await?;
    spotify.wake();
    Ok(())
}

/// Traz o Spotify pra frente mostrando a faixa. Abrir a URI não interrompe nem reinicia a reprodução.
#[tauri::command]
pub async fn spotify_open(uri: String) -> Result<(), String> {
    // só URIs de faixa: o comando não pode virar um "open" genérico
    let valid = uri
        .strip_prefix("spotify:track:")
        .is_some_and(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric()));
    if !valid {
        return Err(format!("URI inválida: {uri}"));
    }
    tauri::async_runtime::spawn_blocking(move || Command::new("open").arg(&uri).status())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
        .and_then(|status| if status.success() { Ok(()) } else { Err(format!("open saiu com {status}")) })
}

#[tauri::command]
pub async fn spotify_shuffle(enabled: bool, spotify: State<'_, Arc<Spotify>>) -> Result<(), String> {
    run(format!(r#"tell application "Spotify" to set shuffling to {enabled}"#)).await?;
    spotify.wake();
    Ok(())
}

#[tauri::command]
pub async fn spotify_repeat(mode: Repeat, spotify: State<'_, Arc<Spotify>>) -> Result<(), String> {
    let current = spotify.last.lock().unwrap().track.as_ref().map(|t| t.id.clone());
    spotify.set_repeat_track(if mode == Repeat::Track { current } else { None });
    // no "repetir faixa" o Spotify fica em "repetir tudo": se a emulação falhar, ao menos não para
    let on = mode != Repeat::Off;
    run(format!(r#"tell application "Spotify" to set repeating to {on}"#)).await?;
    spotify.wake();
    Ok(())
}
