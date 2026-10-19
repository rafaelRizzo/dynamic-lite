//! GIF do Equalizer: o arquivo fica na pasta de configuração; nas settings só vão o tamanho e a versão.
//! O seletor é o diálogo nativo (o `<input type="file">` do WebKit ignora o filtro de tipo).

use std::fs;
use std::path::PathBuf;

use tauri::ipc::Response;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::settings::{self, SharedSettings};

/// Limite do arquivo: 50 MB.
pub const MAX_BYTES: u64 = 50 * 1024 * 1024;
const FILE: &str = "equalizer.gif";

fn path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_config_dir().map(|dir| dir.join(FILE)).map_err(|e| e.to_string())
}

pub fn exists(app: &AppHandle) -> bool {
    path(app).is_ok_and(|p| p.is_file())
}

fn mb(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / 1024.0 / 1024.0).replace('.', ",")
}

/// Abre o seletor (só `.gif`), valida e salva. `Ok(false)`: o usuário cancelou.
#[tauri::command]
pub async fn equalizer_gif_pick(app: AppHandle, settings: State<'_, SharedSettings>) -> Result<bool, String> {
    let Some(picked) = app.dialog().file().add_filter("GIF", &["gif"]).blocking_pick_file() else {
        return Ok(false);
    };
    let source = picked.into_path().map_err(|e| e.to_string())?;

    // o tamanho é checado antes de ler: um arquivo enorme não vai pra memória
    let size = fs::metadata(&source).map_err(|e| e.to_string())?.len();
    if size > MAX_BYTES {
        return Err(format!("O GIF tem {} MB. O limite é {} MB.", mb(size), MAX_BYTES / 1024 / 1024));
    }
    let data = fs::read(&source).map_err(|e| e.to_string())?;
    // extensão não prova nada (um PNG renomeado passa pelo filtro): confere a assinatura
    if !(data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a")) {
        return Err("Formato não suportado. Envie um arquivo GIF.".into());
    }
    // o arquivo pode ter crescido entre o metadata e a leitura
    if data.len() as u64 > MAX_BYTES {
        return Err(format!("O GIF tem {} MB. O limite é {} MB.", mb(data.len() as u64), MAX_BYTES / 1024 / 1024));
    }

    let target = path(&app)?;
    if let Some(dir) = target.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    // grava num temporário e renomeia: uma falha no meio não deixa um GIF pela metade
    let tmp = target.with_extension("gif.tmp");
    fs::write(&tmp, &data).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &target).map_err(|e| e.to_string())?;
    settings::set_gif(&app, &settings, data.len() as u64)?;
    Ok(true)
}

#[tauri::command]
pub fn equalizer_gif_remove(app: AppHandle, settings: State<'_, SharedSettings>) -> Result<(), String> {
    let _ = fs::remove_file(path(&app)?);
    settings::set_gif(&app, &settings, 0)
}

#[tauri::command]
pub fn equalizer_gif_read(app: AppHandle) -> Result<Response, String> {
    fs::read(path(&app)?).map(Response::new).map_err(|e| e.to_string())
}
