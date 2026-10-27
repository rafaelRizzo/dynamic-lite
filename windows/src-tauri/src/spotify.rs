//! Now Playing do Spotify via System Media Transport Controls do Windows (a mesma fonte do overlay de mídia do
//! sistema). Faixa, capa, posição, shuffle e repeat vêm da sessão; o volume, do mixer de áudio (`volume.rs`).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
};
use windows::Media::MediaPlaybackAutoRepeatMode;
use windows::core::Interface;

use crate::{equalizer, gsmtc, volume, win};

pub const STATE_EVENT: &str = "spotify://state";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[derive(Clone, Copy, Default, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Closed,
    Stopped,
    Playing,
    Paused,
}

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
}

impl Spotify {
    pub fn new(waker: Sender<()>) -> Arc<Self> {
        Arc::new(Self { last: Mutex::default(), waker: Mutex::new(waker) })
    }

    /// Força uma leitura imediata no watcher.
    pub fn wake(&self) {
        let _ = self.waker.lock().unwrap().send(());
    }

    fn waker(&self) -> Sender<()> {
        self.waker.lock().unwrap().clone()
    }
}

/// Capa já lida: a leitura é um round-trip ao sistema, então só se repete quando a faixa muda ou a capa ainda não chegou.
#[derive(Default)]
struct Art {
    track: String,
    url: Option<String>,
}

fn track_id(title: &str, artist: &str, album: &str) -> String {
    let mut hasher = DefaultHasher::new();
    (title, artist, album).hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn read_session(session: &Session, art: &mut Art, volume: u8) -> windows::core::Result<PlayerState> {
    let info = session.GetPlaybackInfo()?;
    let status = match info.PlaybackStatus()? {
        PlaybackStatus::Playing => Status::Playing,
        PlaybackStatus::Paused => Status::Paused,
        _ => Status::Stopped,
    };
    let props = session.TryGetMediaPropertiesAsync()?.join()?;
    let (title, artist, album) = (props.Title()?.to_string(), props.Artist()?.to_string(), props.AlbumTitle()?.to_string());
    if title.is_empty() {
        return Ok(PlayerState { status: Status::Stopped, ..Default::default() });
    }

    let id = track_id(&title, &artist, &album);
    if art.track != id || art.url.is_none() {
        art.url = gsmtc::thumbnail(&props);
        art.track = id.clone();
    }

    let timeline = session.GetTimelineProperties()?;
    let (start, end) = (timeline.StartTime()?.Duration, timeline.EndTime()?.Duration);
    let duration = ((end - start).max(0) as f64) / gsmtc::TICKS;
    let mut position = ((timeline.Position()?.Duration - start).max(0) as f64) / gsmtc::TICKS;
    if status == Status::Playing {
        // a posição é a de `LastUpdatedTime`: avança até agora
        let elapsed = (gsmtc::now_ticks() - timeline.LastUpdatedTime()?.UniversalTime).max(0) as f64 / gsmtc::TICKS;
        position += elapsed;
    }
    if duration > 0.0 {
        position = position.min(duration);
    }

    let shuffle = info.IsShuffleActive().and_then(|v| v.Value()).unwrap_or(false);
    let repeat = match info.AutoRepeatMode().and_then(|v| v.Value()) {
        Ok(MediaPlaybackAutoRepeatMode::Track) => Repeat::Track,
        Ok(MediaPlaybackAutoRepeatMode::List) => Repeat::Context,
        _ => Repeat::Off,
    };

    Ok(PlayerState {
        status,
        track: Some(Track { id, name: title, artist, album, artwork_url: art.url.clone().unwrap_or_default(), duration }),
        position,
        volume,
        shuffle,
        repeat,
    })
}

fn read_state(manager: &Manager, art: &mut Art, volume: u8) -> (PlayerState, Option<Session>) {
    let Some(session) = gsmtc::spotify_session(manager) else { return (PlayerState::default(), None) };
    let state = read_session(&session, art, volume).unwrap_or(PlayerState { status: Status::Stopped, ..Default::default() });
    (state, Some(session))
}

/// A leitura nova só vale um evento se algo mudou além do avanço normal da posição (play/pause, faixa, seek, volume...).
fn changed(last: &PlayerState, next: &PlayerState, since: Duration) -> bool {
    let mut expected = last.clone();
    if last.status == Status::Playing {
        expected.position += since.as_secs_f64();
    }
    let drift = (next.position - expected.position).abs();
    expected.position = next.position;
    expected != *next || drift > 1.5
}

/// Lê o estado quando acordado (evento do sistema ou comando) ou num polling curto de segurança.
pub fn spawn_watcher(app: AppHandle, spotify: Arc<Spotify>, rx: Receiver<()>, equalizer: Sender<equalizer::Msg>) {
    thread::spawn(move || {
        win::com_init();
        let mut manager = gsmtc::manager().ok();
        let mut art = Art::default();
        let mut subscribed: usize = 0;
        let mut emitted_at = Instant::now();
        loop {
            if manager.is_none() {
                manager = gsmtc::manager().ok();
            }
            let volume = volume::get().unwrap_or(100);
            let (state, session) = manager.as_ref().map_or((PlayerState::default(), None), |m| read_state(m, &mut art, volume));

            // acorda na hora em que o Spotify troca de faixa ou de estado, sem esperar o polling
            if let Some(session) = &session {
                let ptr = session.as_raw() as usize;
                if ptr != subscribed {
                    subscribed = ptr;
                    subscribe(session, spotify.waker());
                }
            } else {
                subscribed = 0;
            }

            let interval = match state.status {
                Status::Playing => Duration::from_secs(1),
                Status::Closed => Duration::from_secs(3),
                _ => Duration::from_secs(2),
            };

            let _ = equalizer.send(equalizer::Msg::Spotify(state.status == Status::Playing));
            {
                let mut last = spotify.last.lock().unwrap();
                if changed(&last, &state, emitted_at.elapsed()) {
                    let _ = app.emit(STATE_EVENT, &state);
                    *last = state;
                    emitted_at = Instant::now();
                }
            }

            match rx.recv_timeout(interval) {
                Ok(()) => {
                    // as notificações vêm em rajada (faixa, capa, estado): lê uma vez depois que assentam
                    thread::sleep(Duration::from_millis(120));
                    while rx.try_recv().is_ok() {}
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    });
}

fn subscribe(session: &Session, tx: Sender<()>) {
    let media = tx.clone();
    let _ = session.MediaPropertiesChanged(&TypedEventHandler::new(move |_, _| {
        let _ = media.send(());
        Ok(())
    }));
    let _ = session.PlaybackInfoChanged(&TypedEventHandler::new(move |_, _| {
        let _ = tx.send(());
        Ok(())
    }));
}

/// Roda `f` com a sessão do Spotify numa thread de trabalho (as chamadas do WinRT bloqueiam).
async fn with_session<R: Send + 'static>(f: impl FnOnce(Session) -> windows::core::Result<R> + Send + 'static) -> Result<Option<R>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let manager = gsmtc::manager().map_err(|e| e.message())?;
        match gsmtc::spotify_session(&manager) {
            Some(session) => f(session).map(Some).map_err(|e| e.message()),
            None => Ok(None),
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn spotify_state(spotify: State<'_, Arc<Spotify>>) -> PlayerState {
    spotify.last.lock().unwrap().clone()
}

#[tauri::command]
pub async fn spotify_control(action: String, spotify: State<'_, Arc<Spotify>>) -> Result<(), String> {
    let action = match action.as_str() {
        "playpause" | "next" | "previous" => action,
        other => return Err(format!("ação desconhecida: {other}")),
    };
    with_session(move |s| {
        match action.as_str() {
            "playpause" => s.TryTogglePlayPauseAsync()?.join()?,
            "next" => s.TrySkipNextAsync()?.join()?,
            _ => s.TrySkipPreviousAsync()?.join()?,
        };
        Ok(())
    })
    .await?;
    spotify.wake();
    Ok(())
}

#[tauri::command]
pub async fn spotify_seek(position: f64, spotify: State<'_, Arc<Spotify>>) -> Result<(), String> {
    let ticks = (position.max(0.0) * gsmtc::TICKS) as i64;
    with_session(move |s| s.TryChangePlaybackPositionAsync(ticks)?.join().map(|_| ())).await?;
    spotify.wake();
    Ok(())
}

#[tauri::command]
pub async fn spotify_volume(volume: u8, spotify: State<'_, Arc<Spotify>>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || volume::set(volume)).await.map_err(|e| e.to_string())?;
    spotify.wake();
    Ok(())
}

/// Traz o Spotify pra frente. O Windows não expõe o id da faixa, então abre o app (sem interromper a reprodução).
#[tauri::command]
pub async fn spotify_open(uri: String) -> Result<(), String> {
    let valid = uri
        .strip_prefix("spotify:track:")
        .is_some_and(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric()));
    if !valid {
        return Err(format!("URI inválida: {uri}"));
    }
    tauri::async_runtime::spawn_blocking(|| {
        Command::new("cmd").args(["/C", "start", "", "spotify:"]).creation_flags(CREATE_NO_WINDOW).status()
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
    .and_then(|status| if status.success() { Ok(()) } else { Err(format!("start saiu com {status}")) })
}

#[tauri::command]
pub async fn spotify_shuffle(enabled: bool, spotify: State<'_, Arc<Spotify>>) -> Result<(), String> {
    with_session(move |s| s.TryChangeShuffleActiveAsync(enabled)?.join().map(|_| ())).await?;
    spotify.wake();
    Ok(())
}

#[tauri::command]
pub async fn spotify_repeat(mode: Repeat, spotify: State<'_, Arc<Spotify>>) -> Result<(), String> {
    let mode = match mode {
        Repeat::Off => MediaPlaybackAutoRepeatMode::None,
        Repeat::Context => MediaPlaybackAutoRepeatMode::List,
        Repeat::Track => MediaPlaybackAutoRepeatMode::Track,
    };
    with_session(move |s| s.TryChangeAutoRepeatModeAsync(mode)?.join().map(|_| ())).await?;
    spotify.wake();
    Ok(())
}
