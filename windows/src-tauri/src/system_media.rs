//! Metadados do áudio do sistema (ADR 0010): lê a sessão de mídia do Windows que está tocando (navegador, player...),
//! a mesma que o overlay de mídia do sistema mostra. Só roda enquanto há som audível.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus;

use crate::{equalizer, gsmtc, win};

pub const EVENT: &str = "equalizer://media";
const POLL: Duration = Duration::from_secs(2);
const IDLE: Duration = Duration::from_secs(1);

#[derive(Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SystemMedia {
    /// app que está com o áudio (navegador, player...)
    pub app: String,
    /// faixa ou vídeo tocando
    pub title: Option<String>,
    /// artista ou canal, quando o app informa
    pub site: Option<String>,
    pub artwork_url: Option<String>,
}

pub type Current = Arc<Mutex<Option<SystemMedia>>>;

/// Capa da última mídia lida: só se busca de novo quando muda a faixa ou a capa ainda não chegou.
#[derive(Default)]
struct Art {
    key: String,
    url: Option<String>,
}

fn non_empty(s: String) -> Option<String> {
    let s = s.trim().to_owned();
    (!s.is_empty()).then_some(s)
}

/// A primeira sessão de mídia tocando que não seja o Spotify (ele tem a própria Activity).
fn read(art: &mut Art) -> Option<SystemMedia> {
    let manager = gsmtc::manager().ok()?;
    let session = manager.GetSessions().ok()?.into_iter().find(|s| {
        !gsmtc::is_spotify(s)
            && s.GetPlaybackInfo().and_then(|i| i.PlaybackStatus()).is_ok_and(|p| p == PlaybackStatus::Playing)
    })?;
    let app = gsmtc::app_label(&gsmtc::app_id(&session));
    let props = session.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
    let title = non_empty(props.Title().ok()?.to_string());
    let site = props.Artist().ok().map(|a| a.to_string()).and_then(non_empty);

    let key = format!("{app}\n{}\n{}", title.as_deref().unwrap_or_default(), site.as_deref().unwrap_or_default());
    if art.key != key || art.url.is_none() {
        art.url = gsmtc::thumbnail(&props);
        art.key = key;
    }
    Some(SystemMedia { app, title, site, artwork_url: art.url.clone() })
}

#[tauri::command]
pub fn system_media(current: State<'_, Current>) -> Option<SystemMedia> {
    current.lock().unwrap().clone()
}

pub fn spawn(app: AppHandle, current: Current) {
    thread::spawn(move || {
        win::com_init();
        let mut art = Art::default();
        loop {
            let media = if equalizer::system_active() { read(&mut art) } else { None };
            {
                let mut last = current.lock().unwrap();
                if *last != media {
                    let _ = app.emit(EVENT, &media);
                    *last = media.clone();
                }
            }
            thread::sleep(if media.is_some() { POLL } else { IDLE });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_fields_are_none() {
        assert_eq!(non_empty("  ".into()), None);
        assert_eq!(non_empty(" Canal ".into()), Some("Canal".into()));
    }
}
