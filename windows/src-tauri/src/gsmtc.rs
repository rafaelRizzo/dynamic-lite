//! Sessões de mídia do Windows (System Media Transport Controls): a mesma fonte do controle de volume/mídia do
//! sistema. Spotify, navegadores e players publicam faixa, capa, posição e aceitam play/pause/pular.

use base64::engine::general_purpose::STANDARD;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use base64::Engine;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionMediaProperties as Properties,
};
use windows::Storage::Streams::DataReader;
use windows::Win32::Foundation::E_FAIL;

use crate::win;

/// Maior capa aceita: acima disso é lixo ou ataque, não arte de álbum.
const MAX_ART_BYTES: u32 = 4 * 1024 * 1024;

/// Ticks (100 ns) por segundo, a unidade de `TimeSpan` e `DateTime` do WinRT.
pub const TICKS: f64 = 10_000_000.0;
/// Segundos entre 1601-01-01 (época do WinRT) e 1970-01-01.
const EPOCH_GAP_SECS: i64 = 11_644_473_600;

/// Depois de uma falha, só se pede um Manager novo passado este tempo.
const RETRY: Duration = Duration::from_secs(30);

struct Shared {
    manager: Option<Manager>,
    requested_at: Option<Instant>,
}

static SHARED: Mutex<Shared> = Mutex::new(Shared { manager: None, requested_at: None });

/// O Manager de sessões de mídia do processo: pedido uma vez e reaproveitado por todas as threads.
///
/// Cada `RequestAsync` deixa um registro no serviço de sessões do Windows que nunca é liberado, nem ao soltar o
/// objeto. O serviço avisa todos os registros a cada mudança de qualquer sessão: pedir um Manager por leitura
/// (a cada 1-2 s) acumulava milhares deles em poucas horas e mantinha o serviço e o app queimando CPU.
pub fn manager() -> windows::core::Result<Manager> {
    win::com_init();
    let mut shared = SHARED.lock().unwrap();
    if let Some(manager) = &shared.manager {
        return Ok(manager.clone());
    }
    if shared.requested_at.is_some_and(|at| at.elapsed() < RETRY) {
        return Err(E_FAIL.into());
    }
    shared.requested_at = Some(Instant::now());
    let manager = Manager::RequestAsync()?.join()?;
    shared.manager = Some(manager.clone());
    Ok(manager)
}

/// Sessões de mídia abertas. Se o sistema recusar (serviço reiniciado), o Manager guardado é descartado e o
/// próximo `manager()` pede outro, respeitando o intervalo de `RETRY`.
pub fn sessions(manager: &Manager) -> Vec<Session> {
    match manager.GetSessions() {
        Ok(list) => list.into_iter().collect(),
        Err(_) => {
            SHARED.lock().unwrap().manager = None;
            Vec::new()
        }
    }
}

pub fn app_id(session: &Session) -> String {
    session.SourceAppUserModelId().map(|s| s.to_string()).unwrap_or_default()
}

pub fn is_spotify(session: &Session) -> bool {
    app_id(session).to_ascii_lowercase().contains("spotify")
}

pub fn spotify_session(manager: &Manager) -> Option<Session> {
    sessions(manager).into_iter().find(is_spotify)
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

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::Interface;

    /// Precisa do serviço de sessões de mídia (não existe no runner do CI): `cargo test manager_is_shared -- --ignored`
    #[test]
    #[ignore]
    fn manager_is_shared() {
        let first = manager().unwrap();
        let other = std::thread::spawn(|| manager().unwrap().as_raw() as usize).join().unwrap();
        assert_eq!(first.as_raw() as usize, other);
    }
}
