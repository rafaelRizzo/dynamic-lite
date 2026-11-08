//! Letra da música tocando, buscada na LRCLIB (lrclib.net): gratuita, sem chave, com letra sincronizada em LRC.
//! A busca roda aqui, não no webview, pra o CSP do frontend continuar sem acesso à rede. Só é chamada com a opção
//! "Letra" ligada e o painel de letra aberto; o resultado fica em cache por faixa.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};

const API: &str = "https://lrclib.net/api";
const USER_AGENT: &str = concat!("DynamicLite/", env!("CARGO_PKG_VERSION"), " (https://github.com/rafaelRizzo/dynamic-lite)");
const TIMEOUT: Duration = Duration::from_secs(8);
/// Faixas guardadas em cache; ao encher, recomeça do zero (a letra da faixa atual é buscada de novo, só).
const CACHE_MAX: usize = 40;
/// Diferença máxima de duração (s) pra uma letra da busca contar como da mesma gravação.
const DURATION_SLACK: f64 = 3.0;

/// `synced_lyrics` é o texto LRC (`[mm:ss.xx] linha`); o frontend interpreta.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Lyrics {
    pub synced_lyrics: Option<String>,
    pub plain_lyrics: Option<String>,
    /// só na busca: duração da gravação, em segundos
    #[serde(skip_serializing)]
    pub duration: Option<f64>,
}

impl Lyrics {
    fn has_text(&self) -> bool {
        let filled = |s: &Option<String>| s.as_deref().is_some_and(|t| !t.trim().is_empty());
        filled(&self.synced_lyrics) || filled(&self.plain_lyrics)
    }
}

type Cache = Mutex<HashMap<String, Option<Lyrics>>>;

fn cache() -> &'static Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(Mutex::default)
}

/// 404 é "não existe" (resposta definitiva); qualquer outra falha é erro (rede fora, serviço instável).
fn call(request: ureq::Request) -> Result<Option<ureq::Response>, String> {
    match request.set("User-Agent", USER_AGENT).timeout(TIMEOUT).call() {
        Ok(response) => Ok(Some(response)),
        Err(ureq::Error::Status(404, _)) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Primeiro artista de "A, B": a LRCLIB casa melhor só com ele.
fn first_artist(artist: &str) -> &str {
    artist.split([',', '&']).next().unwrap_or(artist).trim()
}

/// A melhor letra da busca: da mesma duração e sincronizada primeiro, depois qualquer sincronizada, depois a primeira.
fn best(hits: Vec<Lyrics>, duration: f64) -> Option<Lyrics> {
    let usable: Vec<Lyrics> = hits.into_iter().filter(Lyrics::has_text).collect();
    let synced = |l: &&Lyrics| l.synced_lyrics.as_deref().is_some_and(|t| !t.trim().is_empty());
    let close = |l: &&Lyrics| l.duration.is_some_and(|d| (d - duration).abs() <= DURATION_SLACK);
    usable
        .iter()
        .find(|l| synced(l) && close(l))
        .or_else(|| usable.iter().find(synced))
        .or_else(|| usable.first())
        .cloned()
}

fn fetch(title: &str, artist: &str, album: &str, duration: f64) -> Result<Option<Lyrics>, String> {
    let exact = call(
        ureq::get(&format!("{API}/get"))
            .query("track_name", title)
            .query("artist_name", artist)
            .query("album_name", album)
            .query("duration", &(duration.round() as i64).to_string()),
    )?;
    if let Some(response) = exact {
        let lyrics: Lyrics = response.into_json().map_err(|e| e.to_string())?;
        if lyrics.has_text() {
            return Ok(Some(lyrics));
        }
    }
    // sem casamento exato (álbum diferente, duração fora): procura só por título e artista
    let found = call(
        ureq::get(&format!("{API}/search")).query("track_name", title).query("artist_name", first_artist(artist)),
    )?;
    let Some(response) = found else { return Ok(None) };
    let hits: Vec<Lyrics> = response.into_json().map_err(|e| e.to_string())?;
    Ok(best(hits, duration))
}

#[tauri::command]
pub async fn lyrics_get(title: String, artist: String, album: String, duration: f64) -> Result<Option<Lyrics>, String> {
    let key = format!("{artist}\n{title}\n{album}\n{}", duration.round());
    if let Some(hit) = cache().lock().unwrap().get(&key) {
        return Ok(hit.clone());
    }
    let result = tauri::async_runtime::spawn_blocking(move || {
        let lyrics = fetch(&title, &artist, &album, duration)?;
        let mut cache = cache().lock().unwrap();
        if cache.len() >= CACHE_MAX {
            cache.clear();
        }
        cache.insert(key, lyrics.clone());
        Ok(lyrics)
    })
    .await
    .map_err(|e| e.to_string())?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(synced: Option<&str>, plain: Option<&str>, duration: f64) -> Lyrics {
        Lyrics { synced_lyrics: synced.map(Into::into), plain_lyrics: plain.map(Into::into), duration: Some(duration) }
    }

    #[test]
    fn first_artist_takes_the_main_one() {
        assert_eq!(first_artist("Daft Punk, Pharrell Williams"), "Daft Punk");
        assert_eq!(first_artist("Simon & Garfunkel"), "Simon");
        assert_eq!(first_artist("Queen"), "Queen");
    }

    #[test]
    fn best_prefers_synced_with_matching_duration() {
        let hits = vec![
            hit(None, Some("só texto"), 200.0),
            hit(Some("[00:01.00] outra gravação"), None, 260.0),
            hit(Some("[00:01.00] certa"), None, 201.0),
        ];
        assert_eq!(best(hits, 200.0).unwrap().synced_lyrics.as_deref(), Some("[00:01.00] certa"));
    }

    #[test]
    fn best_falls_back_to_any_synced_then_plain() {
        let hits = vec![hit(None, Some("só texto"), 200.0), hit(Some("[00:01.00] x"), None, 999.0)];
        assert_eq!(best(hits, 200.0).unwrap().synced_lyrics.as_deref(), Some("[00:01.00] x"));
        assert_eq!(best(vec![hit(None, Some("só texto"), 1.0)], 200.0).unwrap().plain_lyrics.as_deref(), Some("só texto"));
    }

    #[test]
    fn best_ignores_empty_lyrics() {
        assert_eq!(best(vec![hit(Some("  "), Some(""), 200.0)], 200.0), None);
    }
}
