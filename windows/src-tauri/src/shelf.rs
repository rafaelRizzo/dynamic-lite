//! Prateleira de arquivos: o que você solta na aba Arquivos (arquivos, prints) fica guardado numa pasta do app,
//! com uma cópia de cada item, até você remover. Sem limite de tamanho nem de tempo: quem limita é o disco.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, DragDropEvent, Emitter, Manager};

const INDEX: &str = "index.json";
/// `bool`: arquivos do sistema estão sendo arrastados por cima da Island.
pub const DRAG_EVENT: &str = "shelf://drag";
/// A prateleira inteira, depois de soltar arquivos.
pub const CHANGED_EVENT: &str = "shelf://changed";
/// Mensagem pronta quando soltar falhou.
pub const ERROR_EVENT: &str = "shelf://error";
/// Serializa leitura e escrita do índice (comandos rodam em threads diferentes).
static LOCK: Mutex<()> = Mutex::new(());

const AUDIO: &[&str] = &["mp3", "wav", "wave", "flac", "ogg", "oga", "m4a", "aac", "mp4", "gsm", "wav49", "mka"];
const IMAGE: &[&str] = &["png", "jpg", "jpeg", "gif", "webp", "bmp"];

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Audio,
    Image,
    File,
}

fn kind_of(name: &str) -> Kind {
    let ext = Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    if AUDIO.contains(&ext.as_str()) {
        Kind::Audio
    } else if IMAGE.contains(&ext.as_str()) {
        Kind::Image
    } else {
        Kind::File
    }
}

/// O que fica no índice. O caminho completo é calculado na hora (a pasta do app pode mudar de lugar).
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Stored {
    id: String,
    name: String,
    /// nome do arquivo guardado dentro da prateleira
    stored: String,
    size: u64,
    kind: Kind,
    /// ms desde a época
    added_at: u64,
    /// de onde veio (vazio em prints colados)
    origin: Option<String>,
}

#[derive(Clone, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub name: String,
    pub path: String,
    pub size: u64,
    pub kind: Kind,
    pub added_at: u64,
    /// pasta do arquivo original, se ainda existe: onde o conversor grava
    pub out_dir: Option<String>,
}

fn dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("shelf");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn read_index(dir: &Path) -> Vec<Stored> {
    fs::read_to_string(dir.join(INDEX)).ok().and_then(|raw| serde_json::from_str(&raw).ok()).unwrap_or_default()
}

fn write_index(dir: &Path, items: &[Stored]) -> Result<(), String> {
    let json = serde_json::to_string_pretty(items).map_err(|e| e.to_string())?;
    fs::write(dir.join(INDEX), json).map_err(|e| e.to_string())
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}

/// Nome sem pastas nem caracteres que o Windows recusa.
fn safe_name(name: &str) -> String {
    let cleaned: String = name.chars().map(|c| if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control() { '_' } else { c }).collect();
    let cleaned = cleaned.trim().trim_matches('.').to_owned();
    if cleaned.is_empty() { "arquivo".into() } else { cleaned }
}

fn view(dir: &Path, s: &Stored) -> Item {
    let out_dir = s
        .origin
        .as_deref()
        .and_then(|o| Path::new(o).parent())
        .filter(|p| p.is_dir())
        .map(|p| p.to_string_lossy().into_owned());
    Item {
        id: s.id.clone(),
        name: s.name.clone(),
        path: dir.join(&s.stored).to_string_lossy().into_owned(),
        size: s.size,
        kind: s.kind,
        added_at: s.added_at,
        out_dir,
    }
}

/// Itens cujo arquivo guardado ainda existe, do mais novo pro mais antigo.
fn items(dir: &Path) -> Vec<Stored> {
    let mut all: Vec<Stored> = read_index(dir).into_iter().filter(|s| dir.join(&s.stored).is_file()).collect();
    all.sort_by(|a, b| b.added_at.cmp(&a.added_at));
    all
}

fn unique_id() -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    format!("{:x}{:x}", now_ms(), COUNTER.fetch_add(1, Ordering::Relaxed))
}

/// Copia `src` pra prateleira e devolve o registro (sem gravar o índice).
fn store_file(dir: &Path, src: &Path) -> Result<Stored, String> {
    let mut name = safe_name(src.file_name().and_then(|n| n.to_str()).unwrap_or("arquivo"));
    // copiar uma cópia da própria prateleira (`<id>-nome`) não acumula o prefixo de id no nome
    if src.parent() == Some(dir) {
        while let Some((head, rest)) = name.split_once('-') {
            if head.len() < 8 || !head.chars().all(|c| c.is_ascii_hexdigit()) || rest.is_empty() {
                break;
            }
            name = rest.to_string();
        }
    }
    let id = unique_id();
    let stored = format!("{id}-{name}");
    let size = fs::copy(src, dir.join(&stored)).map_err(|e| format!("{name}: {e}"))?;
    Ok(Stored { id, kind: kind_of(&name), name, stored, size, added_at: now_ms(), origin: Some(src.to_string_lossy().into_owned()) })
}

#[tauri::command]
pub fn shelf_list(app: AppHandle) -> Result<Vec<Item>, String> {
    let _guard = LOCK.lock().unwrap();
    let dir = dir(&app)?;
    Ok(items(&dir).iter().map(|s| view(&dir, s)).collect())
}

/// Copia os arquivos (pastas são ignoradas) e devolve a prateleira inteira.
fn add_paths(app: &AppHandle, paths: &[String]) -> Result<Vec<Item>, String> {
    let _guard = LOCK.lock().unwrap();
    let dir = dir(app)?;
    let mut all = items(&dir);
    let mut failed = Vec::new();
    for path in paths.iter().map(PathBuf::from).filter(|p| p.is_file()) {
        match store_file(&dir, &path) {
            Ok(item) => all.push(item),
            Err(e) => failed.push(e),
        }
    }
    write_index(&dir, &all)?;
    all.sort_by(|a, b| b.added_at.cmp(&a.added_at));
    if !failed.is_empty() {
        return Err(failed.join("; "));
    }
    Ok(all.iter().map(|s| view(&dir, s)).collect())
}

#[tauri::command]
pub async fn shelf_add(app: AppHandle, paths: Vec<String>) -> Result<Vec<Item>, String> {
    tauri::async_runtime::spawn_blocking(move || add_paths(&app, &paths)).await.map_err(|e| e.to_string())?
}

/// Arrasto de arquivos do sistema por cima da janela. Tratado aqui, no Rust, porque o evento chega mesmo com a
/// janela sem foco, sem depender de o frontend ter assinado o evento do Tauri.
pub fn on_drag_drop(app: &AppHandle, event: &DragDropEvent) {
    match event {
        DragDropEvent::Enter { .. } => {
            let _ = app.emit(DRAG_EVENT, true);
        }
        DragDropEvent::Leave => {
            let _ = app.emit(DRAG_EVENT, false);
        }
        DragDropEvent::Drop { paths, .. } => {
            let _ = app.emit(DRAG_EVENT, false);
            let app = app.clone();
            let paths: Vec<String> = paths.iter().map(|p| p.to_string_lossy().into_owned()).collect();
            std::thread::spawn(move || match add_paths(&app, &paths) {
                Ok(list) => {
                    let _ = app.emit(CHANGED_EVENT, list);
                }
                Err(message) => {
                    let _ = app.emit(ERROR_EVENT, message);
                    // o que deu certo antes do erro continua na prateleira
                    if let Ok(list) = shelf_list(app.clone()) {
                        let _ = app.emit(CHANGED_EVENT, list);
                    }
                }
            });
        }
        _ => {}
    }
}

/// Abre o seletor de arquivos do sistema (começa em Downloads) e guarda o que foi escolhido.
#[tauri::command]
pub async fn shelf_pick(app: AppHandle) -> Result<Vec<Item>, String> {
    use tauri_plugin_dialog::DialogExt;
    tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = app.dialog().file().set_title("Adicionar à prateleira");
        if let Ok(downloads) = app.path().download_dir() {
            dialog = dialog.set_directory(downloads);
        }
        let Some(picked) = dialog.blocking_pick_files() else {
            return shelf_list(app.clone());
        };
        let paths: Vec<String> = picked.into_iter().filter_map(|f| f.into_path().ok()).map(|p| p.to_string_lossy().into_owned()).collect();
        add_paths(&app, &paths)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Cola da área de transferência: arquivos copiados no explorador (Ctrl+C) ou uma imagem (um print, por exemplo) como PNG.
#[tauri::command]
pub async fn shelf_paste_image(app: AppHandle) -> Result<Vec<Item>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        if let Ok(files) = clipboard.get().file_list() {
            let paths: Vec<String> = files.iter().map(|p| p.to_string_lossy().into_owned()).collect();
            if paths.iter().any(|p| std::path::Path::new(p).is_file()) {
                return add_paths(&app, &paths);
            }
        }
        let image = clipboard
            .get_image()
            .map_err(|_| "Não há arquivo nem imagem na área de transferência".to_string())?;
        let (width, height) = (image.width as u32, image.height as u32);

        let _guard = LOCK.lock().unwrap();
        let dir = dir(&app)?;
        let id = unique_id();
        let name = format!("print-{}.png", stamp());
        let stored = format!("{id}-{name}");
        {
            let file = fs::File::create(dir.join(&stored)).map_err(|e| e.to_string())?;
            let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
            writer.write_image_data(&image.bytes).map_err(|e| e.to_string())?;
        }
        let size = fs::metadata(dir.join(&stored)).map_or(0, |m| m.len());
        let mut all = items(&dir);
        all.push(Stored { id, name, stored, size, kind: Kind::Image, added_at: now_ms(), origin: None });
        write_index(&dir, &all)?;
        all.sort_by(|a, b| b.added_at.cmp(&a.added_at));
        Ok(all.iter().map(|s| view(&dir, s)).collect())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// `AAAAMMDD-HHMMSS` em UTC (sem depender de fuso nem de crate de datas).
fn stamp() -> String {
    let secs = now_ms() / 1000;
    let (days, rem) = (secs / 86_400, secs % 86_400);
    // dias desde 1970 → data civil (algoritmo de Howard Hinnant)
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}{month:02}{day:02}-{:02}{:02}{:02}", rem / 3600, rem % 3600 / 60, rem % 60)
}

#[tauri::command]
pub fn shelf_remove(app: AppHandle, id: String) -> Result<Vec<Item>, String> {
    let _guard = LOCK.lock().unwrap();
    let dir = dir(&app)?;
    let mut all = items(&dir);
    if let Some(at) = all.iter().position(|s| s.id == id) {
        let gone = all.remove(at);
        let _ = fs::remove_file(dir.join(&gone.stored));
    }
    write_index(&dir, &all)?;
    Ok(all.iter().map(|s| view(&dir, s)).collect())
}

#[tauri::command]
pub fn shelf_clear(app: AppHandle) -> Result<Vec<Item>, String> {
    let _guard = LOCK.lock().unwrap();
    let dir = dir(&app)?;
    for s in items(&dir) {
        let _ = fs::remove_file(dir.join(&s.stored));
    }
    write_index(&dir, &[])?;
    Ok(Vec::new())
}

fn stored_path(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    let _guard = LOCK.lock().unwrap();
    let dir = dir(app)?;
    items(&dir).into_iter().find(|s| s.id == id).map(|s| dir.join(s.stored)).ok_or_else(|| "Item não encontrado".to_string())
}

fn open_path(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    let result = std::process::Command::new("explorer").arg(path).spawn();
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(path).spawn();
    #[cfg(not(any(windows, target_os = "macos")))]
    let result = std::process::Command::new("xdg-open").arg(path).spawn();
    result.map(|_| ()).map_err(|e| e.to_string())
}

fn reveal_path(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    let result = {
        use std::os::windows::process::CommandExt;
        // o Explorer só entende `/select,"caminho"` como argumento cru
        std::process::Command::new("explorer").raw_arg(format!("/select,\"{}\"", path.display())).spawn()
    };
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg("-R").arg(path).spawn();
    #[cfg(not(any(windows, target_os = "macos")))]
    let result = std::process::Command::new("xdg-open").arg(path.parent().unwrap_or(path)).spawn();
    result.map(|_| ()).map_err(|e| e.to_string())
}

/// Abre o item (guardado) com o programa padrão.
#[tauri::command]
pub fn shelf_open(app: AppHandle, id: String) -> Result<(), String> {
    open_path(&stored_path(&app, &id)?)
}

/// Mostra o item (guardado) na pasta.
#[tauri::command]
pub fn shelf_reveal(app: AppHandle, id: String) -> Result<(), String> {
    reveal_path(&stored_path(&app, &id)?)
}

/// Mostra qualquer arquivo na pasta (usado pro resultado de uma conversão). Só caminhos que existem.
#[tauri::command]
pub fn reveal_file(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err("O arquivo não existe mais".into());
    }
    reveal_path(&path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_by_extension() {
        assert_eq!(kind_of("a.MP3"), Kind::Audio);
        assert_eq!(kind_of("b.wav49"), Kind::Audio);
        assert_eq!(kind_of("c.PNG"), Kind::Image);
        assert_eq!(kind_of("d.pdf"), Kind::File);
        assert_eq!(kind_of("sem-extensao"), Kind::File);
    }

    #[test]
    fn names_are_made_safe() {
        assert_eq!(safe_name("a:b*c?.mp3"), "a_b_c_.mp3");
        assert_eq!(safe_name("..."), "arquivo");
        assert_eq!(safe_name("  foto.png "), "foto.png");
    }

    #[test]
    fn stamp_has_the_expected_shape() {
        let s = stamp();
        assert_eq!(s.len(), 15);
        assert_eq!(s.as_bytes()[8], b'-');
        assert!(s.starts_with("20"));
    }

    #[test]
    fn store_and_list_roundtrip() {
        let dir = std::env::temp_dir().join(format!("dl-shelf-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let src = dir.join("origem.mp3");
        fs::write(&src, b"abc").unwrap();
        let shelf = dir.join("shelf");
        fs::create_dir_all(&shelf).unwrap();

        let stored = store_file(&shelf, &src).unwrap();
        assert_eq!((stored.kind, stored.size), (Kind::Audio, 3));
        write_index(&shelf, &[stored.clone()]).unwrap();
        let listed = items(&shelf);
        assert_eq!(listed.len(), 1);
        let item = view(&shelf, &listed[0]);
        assert!(item.out_dir.is_some(), "a pasta do original ainda existe");

        // arquivo guardado apagado por fora: o item some da lista
        fs::remove_file(shelf.join(&stored.stored)).unwrap();
        assert!(items(&shelf).is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }
}
