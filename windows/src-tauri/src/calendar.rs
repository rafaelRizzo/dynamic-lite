//! Calendário: no Windows não há uma API de eventos que um app sem identidade de pacote consiga ler, então a
//! Agenda fica indisponível (o frontend esconde a aba por `Platform::calendar_supported`).

use serde::Serialize;

#[derive(Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Access {
    Denied,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalEvent {
    pub id: String,
    pub title: String,
    /// epoch ms
    pub start: f64,
    pub end: f64,
    pub all_day: bool,
    pub calendar: String,
    pub location: Option<String>,
}

pub fn events(_start_ms: f64, _end_ms: f64) -> Vec<CalEvent> {
    vec![]
}

#[tauri::command]
pub fn calendar_access() -> Access {
    Access::Denied
}

#[tauri::command]
pub async fn calendar_request_access() -> Access {
    Access::Denied
}

#[tauri::command]
pub async fn calendar_events(_start: f64, _end: f64) -> Vec<CalEvent> {
    vec![]
}
