//! Atualização automática pelas Releases do GitHub (latest.json gerado pelo tauri-action).

use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::settings::SharedSettings;

pub const AVAILABLE_EVENT: &str = "update://available";
pub const INSTALLING_EVENT: &str = "update://installing";

/// Primeira checagem um pouco depois de abrir, pra não competir com a inicialização.
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(10);
const CHECK_EVERY: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current_version: String,
    pub notes: Option<String>,
}

/// Update encontrado na última checagem, pronto pra instalar.
#[derive(Default)]
pub struct Updates(Mutex<Option<Update>>);

impl Updates {
    pub fn info(&self) -> Option<UpdateInfo> {
        self.0.lock().unwrap().as_ref().map(|u| UpdateInfo {
            version: u.version.clone(),
            current_version: u.current_version.clone(),
            notes: u.body.clone(),
        })
    }
}

async fn check(app: &AppHandle) -> Result<Option<UpdateInfo>, String> {
    let update = app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())?;
    let updates = app.state::<Updates>();
    *updates.0.lock().unwrap() = update;
    let info = updates.info();
    // emitido sempre: cada checagem reexibe o aviso que o usuário dispensou com "Depois"
    let _ = app.emit(AVAILABLE_EVENT, &info);
    crate::menu::refresh_tray(app);
    Ok(info)
}

/// Checa ao abrir e a cada 24h, se "Verificar atualizações automaticamente" estiver ligado.
pub fn spawn_checker(app: AppHandle) {
    thread::spawn(move || {
        thread::sleep(FIRST_CHECK_DELAY);
        loop {
            let enabled = app.state::<SharedSettings>().lock().unwrap().auto_update;
            if enabled {
                let _ = tauri::async_runtime::block_on(check(&app));
            }
            thread::sleep(CHECK_EVERY);
        }
    });
}

/// Baixa, instala e reabre o app.
pub async fn install(app: AppHandle) -> Result<(), String> {
    let update = app.state::<Updates>().0.lock().unwrap().clone().ok_or("nenhuma atualização pendente")?;
    let _ = app.emit(INSTALLING_EVENT, ());
    update.download_and_install(|_, _| {}, || {}).await.map_err(|e| e.to_string())?;
    app.restart();
}

#[tauri::command]
pub fn get_update(updates: tauri::State<'_, Updates>) -> Option<UpdateInfo> {
    updates.info()
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    check(&app).await
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    install(app).await
}
