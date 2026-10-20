//! Metadados do áudio do sistema (ADR 0010): sem MediaRemote, lê a aba do navegador que está tocando por
//! AppleScript (título e, no YouTube, a miniatura). Só roda enquanto há som audível no sistema.

use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::{equalizer, macos};

pub const EVENT: &str = "equalizer://media";
const POLL: Duration = Duration::from_secs(2);
const IDLE: Duration = Duration::from_secs(1);

#[derive(Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SystemMedia {
    /// app que está com o áudio (navegador, player...)
    pub app: String,
    /// aba tocando: título limpo e site (YouTube...); só em navegadores com uma aba reconhecida
    pub title: Option<String>,
    pub site: Option<String>,
    pub artwork_url: Option<String>,
}

pub type Current = Arc<Mutex<Option<SystemMedia>>>;

#[derive(Clone, Copy)]
enum Engine {
    Chromium,
    Safari,
}

/// (bundle id, nome no AppleScript, dialeto)
const BROWSERS: [(&str, &str, Engine); 7] = [
    ("com.google.Chrome", "Google Chrome", Engine::Chromium),
    ("com.microsoft.edgemac", "Microsoft Edge", Engine::Chromium),
    ("com.brave.Browser", "Brave Browser", Engine::Chromium),
    ("company.thebrowser.Browser", "Arc", Engine::Chromium),
    ("com.vivaldi.Vivaldi", "Vivaldi", Engine::Chromium),
    ("com.operasoftware.Opera", "Opera", Engine::Chromium),
    ("com.apple.Safari", "Safari", Engine::Safari),
];

/// Sites de mídia (domínio, nome). Mais específicos primeiro: `music.youtube.com` antes de `youtube.com`.
const SITES: [(&str, &str); 10] = [
    ("music.youtube.com", "YouTube Music"),
    ("youtube.com", "YouTube"),
    ("youtu.be", "YouTube"),
    ("soundcloud.com", "SoundCloud"),
    ("open.spotify.com", "Spotify"),
    ("music.apple.com", "Apple Music"),
    ("deezer.com", "Deezer"),
    ("twitch.tv", "Twitch"),
    ("netflix.com", "Netflix"),
    ("primevideo.com", "Prime Video"),
];

fn script(name: &str, engine: Engine) -> String {
    let (active, title) = match engine {
        Engine::Chromium => ("active tab of front window", "title"),
        Engine::Safari => ("current tab of front window", "name"),
    };
    format!(
        r#"tell application "{name}"
  set t to ASCII character 9
  set n to linefeed
  set r to ""
  try
    set r to (URL of {active}) & t & ({title} of {active})
  end try
  set r to r & n
  repeat with w in windows
    try
      set us to URL of tabs of w
      set ts to {title} of tabs of w
      repeat with i from 1 to count of us
        set r to r & (item i of us) & t & (item i of ts) & n
      end repeat
    end try
  end repeat
  return r
end tell"#
    )
}

struct Tab {
    url: String,
    title: String,
}

fn parse_tab(line: &str) -> Option<Tab> {
    let (url, title) = line.split_once('\t')?;
    Some(Tab { url: url.into(), title: title.into() })
}

/// (host, caminho + query) de uma URL http(s).
fn split_url(url: &str) -> Option<(&str, &str)> {
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://"))?;
    let end = rest.find('/').unwrap_or(rest.len());
    Some((&rest[..end], &rest[end..]))
}

fn site(url: &str) -> Option<&'static str> {
    let (host, path) = split_url(url)?;
    let host = host.trim_start_matches("www.");
    let (domain, name) = SITES.iter().find(|(d, _)| host == *d || host.ends_with(&format!(".{d}")))?;
    // YouTube comum só conta com vídeo aberto, não a home ou a busca
    let video = path.starts_with("/watch") || path.starts_with("/shorts") || path.starts_with("/live");
    (*domain != "youtube.com" || video).then_some(*name)
}

/// ID de vídeo do YouTube (`watch?v=` ou `youtu.be/`); só `[A-Za-z0-9_-]`, que vai direto numa URL de imagem.
fn youtube_id(url: &str) -> Option<&str> {
    let (host, path) = split_url(url)?;
    let id = if host.trim_start_matches("www.") == "youtu.be" {
        path.trim_start_matches('/').split(['?', '&', '#']).next()?
    } else {
        let query = path.split_once('?')?.1;
        query.split(['&', '#']).find_map(|kv| kv.strip_prefix("v="))?
    };
    (!id.is_empty() && id.len() <= 20 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .then_some(id)
}

/// Caractere que não desenha nada (espaços, fillers hangul, zero-width, braille vazio): há vídeos intitulados só com eles.
fn invisible(c: char) -> bool {
    c.is_whitespace() || matches!(c, '\u{115F}' | '\u{1160}' | '\u{3164}' | '\u{FFA0}' | '\u{2800}' | '\u{2060}' | '\u{200B}'..='\u{200F}')
}

/// Tira o contador de notificações `(3) ` e o sufixo do site do título da aba; vazio se não sobrar nada visível.
fn clean_title(title: &str) -> String {
    let t = title.trim();
    let t = match t.strip_prefix('(').and_then(|r| r.split_once(") ")) {
        Some((n, rest)) if n.chars().all(|c| c.is_ascii_digit()) => rest,
        _ => t,
    };
    let t = t.trim_end_matches(" - YouTube Music").trim_end_matches(" - YouTube");
    if t.chars().all(invisible) { String::new() } else { t.trim().to_owned() }
}

/// A aba tocando é a ativa se for de mídia; senão a primeira de mídia (o AppleScript não diz qual é audível).
fn pick_tab(raw: &str) -> Option<(Tab, &'static str)> {
    let mut lines = raw.lines();
    let active = lines.next().and_then(parse_tab);
    active
        .into_iter()
        .chain(lines.filter_map(parse_tab))
        .find_map(|tab| site(&tab.url).map(|s| (tab, s)))
}

fn osascript(script: &str) -> Option<String> {
    let out = Command::new("osascript").args(["-e", script]).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// `com.google.Chrome.helper.Renderer` → `com.google.Chrome`. O áudio do Safari sai do WebKit.GPU.
fn owner(bundle: &str) -> String {
    if bundle.starts_with("com.apple.WebKit") {
        return "com.apple.Safari".into();
    }
    let lower = bundle.to_ascii_lowercase();
    lower.find(".helper").map_or(bundle, |i| &bundle[..i]).to_owned()
}

fn read() -> Option<SystemMedia> {
    let owners: Vec<String> = equalizer::output_bundles().iter().map(|b| owner(b)).collect();
    // navegador primeiro: é o que dá título e miniatura
    let browser = BROWSERS.iter().find(|(id, ..)| owners.iter().any(|o| o == id));
    if let Some(&(id, name, engine)) = browser {
        let app = macos::app_name(id).unwrap_or_else(|| name.to_owned());
        // `tell application` abriria o navegador se estivesse fechado
        let tab = macos::is_app_running(id)
            .then(|| osascript(&script(name, engine)))
            .flatten()
            .and_then(|raw| pick_tab(&raw));
        return Some(match tab {
            Some((tab, site)) => SystemMedia {
                artwork_url: youtube_id(&tab.url).map(|id| format!("https://i.ytimg.com/vi/{id}/mqdefault.jpg")),
                title: Some(clean_title(&tab.title)).filter(|t| !t.is_empty()),
                site: Some(site.to_owned()),
                app,
            },
            None => SystemMedia { app, title: None, site: None, artwork_url: None },
        });
    }
    let app = owners.iter().find_map(|o| macos::app_name(o))?;
    Some(SystemMedia { app, title: None, site: None, artwork_url: None })
}

#[tauri::command]
pub fn system_media(current: State<'_, Current>) -> Option<SystemMedia> {
    current.lock().unwrap().clone()
}

pub fn spawn(app: AppHandle, current: Current) {
    thread::spawn(move || loop {
        let media = if equalizer::system_active() { read() } else { None };
        {
            let mut last = current.lock().unwrap();
            if *last != media {
                let _ = app.emit(EVENT, &media);
                *last = media.clone();
            }
        }
        thread::sleep(if media.is_some() { POLL } else { IDLE });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn youtube_tab_gets_id_and_clean_title() {
        let raw = "https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=3\t(2) Song - YouTube\n";
        let (tab, site) = pick_tab(raw).unwrap();
        assert_eq!(site, "YouTube");
        assert_eq!(youtube_id(&tab.url), Some("dQw4w9WgXcQ"));
        assert_eq!(clean_title(&tab.title), "Song");
    }

    #[test]
    fn picks_first_media_tab_when_active_is_not_media() {
        let raw = "https://example.com/\tExample\nhttps://example.com/\tExample\nhttps://music.youtube.com/watch?v=abc\tX - YouTube Music\n";
        assert_eq!(pick_tab(raw).unwrap().1, "YouTube Music");
    }

    #[test]
    fn invisible_title_is_empty() {
        assert_eq!(clean_title("\u{3164}\u{3164} \u{3164} - YouTube"), "");
    }

    #[test]
    fn youtube_home_is_not_media() {
        assert_eq!(site("https://www.youtube.com/"), None);
        assert_eq!(site("https://youtu.be/abc"), Some("YouTube"));
        assert_eq!(youtube_id("https://youtu.be/abc?t=1"), Some("abc"));
    }

    #[test]
    fn helper_bundles_map_to_their_browser() {
        assert_eq!(owner("com.google.Chrome.helper.Renderer"), "com.google.Chrome");
        assert_eq!(owner("com.apple.WebKit.GPU"), "com.apple.Safari");
    }
}
