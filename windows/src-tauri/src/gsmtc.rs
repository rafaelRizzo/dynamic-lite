//! Sessões de mídia do Windows (System Media Transport Controls): a mesma fonte do controle de volume/mídia do
//! sistema. Spotify, navegadores e players publicam faixa, capa, posição e aceitam play/pause/pular.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionMediaProperties as Properties,
};
use windows::Storage::Streams::DataReader;

use crate::win;

/// Maior capa aceita: acima disso é lixo ou ataque, não arte de álbum.
const MAX_ART_BYTES: u32 = 4 * 1024 * 1024;

/// Ticks (100 ns) por segundo, a unidade de `TimeSpan` e `DateTime` do WinRT.
pub const TICKS: f64 = 10_000_000.0;
/// Segundos entre 1601-01-01 (época do WinRT) e 1970-01-01.
const EPOCH_GAP_SECS: i64 = 11_644_473_600;

pub fn manager() -> windows::core::Result<Manager> {
    win::com_init();
    Manager::RequestAsync()?.join()
}

pub fn app_id(session: &Session) -> String {
    session.SourceAppUserModelId().map(|s| s.to_string()).unwrap_or_default()
}

pub fn is_spotify(session: &Session) -> bool {
    app_id(session).to_ascii_lowercase().contains("spotify")
}

pub fn spotify_session(manager: &Manager) -> Option<Session> {
    manager.GetSessions().ok()?.into_iter().find(is_spotify)
}

/// Agora, na escala de tempo do WinRT (ticks desde 1601).
pub fn now_ticks() -> i64 {
    let since_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64());
    ((since_unix + EPOCH_GAP_SECS as f64) * TICKS) as i64
}

/// Capa como `data:` URL (o CSP já aceita). `None` se a sessão não publicou ou ainda está carregando.
pub fn thumbnail(props: &Properties) -> Option<String> {
    let stream = props.Thumbnail().ok()?.OpenReadAsync().ok()?.join().ok()?;
    let size = u32::try_from(stream.Size().ok()?).ok()?;
    if size == 0 || size > MAX_ART_BYTES {
        return None;
    }
    let reader = DataReader::CreateDataReader(&stream).ok()?;
    reader.LoadAsync(size).ok()?.join().ok()?;
    let mut bytes = vec![0u8; size as usize];
    reader.ReadBytes(&mut bytes).ok()?;
    let mime = stream.ContentType().map(|m| m.to_string()).ok().filter(|m| m.starts_with("image/"));
    Some(format!("data:{};base64,{}", mime.as_deref().unwrap_or("image/jpeg"), STANDARD.encode(bytes)))
}

/// Nome legível do app dono da sessão, a partir do AppUserModelId (`msedge.exe`, `Chrome`, `Microsoft.ZuneMusic_...!App`).
pub fn app_label(aumid: &str) -> String {
    let lower = aumid.to_ascii_lowercase();
    const KNOWN: [(&str, &str); 11] = [
        ("msedge", "Microsoft Edge"),
        ("chrome", "Google Chrome"),
        ("firefox", "Firefox"),
        ("brave", "Brave"),
        ("opera", "Opera"),
        ("vivaldi", "Vivaldi"),
        ("applemusic", "Apple Music"),
        ("itunes", "iTunes"),
        ("zunemusic", "Media Player"),
        ("vlc", "VLC"),
        ("spotify", "Spotify"),
    ];
    if let Some((_, name)) = KNOWN.iter().find(|(key, _)| lower.contains(key)) {
        return (*name).to_owned();
    }
    let base = aumid.rsplit(['!', '\\']).next().unwrap_or(aumid);
    let base = base.strip_suffix(".exe").or_else(|| base.strip_suffix(".EXE")).unwrap_or(base);
    let mut chars = base.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => "App".to_owned(),
    }
}
