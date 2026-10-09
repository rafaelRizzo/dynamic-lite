//! Now Playing do Spotify via AppleScript (ADR 0002).

use std::process::Command;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::Serialize;
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
  return s & n & (id of t) & n & (name of t) & n & (artist of t) & n & (album of t) & n & (artwork url of t) & n & ((duration of t) as integer) & n & ((player position * 1000) as integer) & n & sound volume
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
}

pub struct Spotify {
    last: Mutex<PlayerState>,
    waker: Mutex<Sender<()>>,
}

impl Spotify {
    pub fn new(waker: Sender<()>) -> Arc<Self> {
        Arc::new(Self { last: Mutex::default(), waker: Mutex::new(waker) })
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
    if f.len() < 9 {
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

/// Lê o estado quando acordado (notificação do Spotify ou comando) ou num polling lento de segurança.
pub fn spawn_watcher(app: AppHandle, spotify: Arc<Spotify>, rx: Receiver<()>) {
    thread::spawn(move || loop {
        let state = read_state();
        let interval = match state.status {
            Status::Playing => Duration::from_secs(3),
            Status::Closed => Duration::from_secs(5),
            _ => Duration::from_secs(4),
        };
        {
            let mut last = spotify.last.lock().unwrap();
            if *last != state {
                let _ = app.emit(STATE_EVENT, &state);
                *last = state;
            }
        }
        match rx.recv_timeout(interval) {
            Ok(()) => {
                // o Spotify às vezes ainda não atualizou a posição no instante da notificação
                thread::sleep(Duration::from_millis(120));
                while rx.try_recv().is_ok() {}
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
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
