//! Calendário via EventKit: lê os eventos do app Calendário (iCloud, Google, Exchange já configurados lá).

use std::sync::mpsc;
use std::time::Duration;

use block2::RcBlock;
use objc2::runtime::Bool;
use objc2::sel;
use objc2_event_kit::{EKAuthorizationStatus, EKEntityType, EKEventStore};
use objc2_foundation::{NSDate, NSError, NSObjectProtocol};
use serde::Serialize;
use tauri::AppHandle;

/// Quanto esperar o usuário responder o diálogo de permissão.
const PROMPT_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Access {
    /// ainda não perguntamos
    Undetermined,
    Granted,
    /// negado ou restrito: só dá pra mudar em Privacidade
    Denied,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalEvent {
    /// instâncias de um evento recorrente repetem o id; a chave única é `id` + `start`
    pub id: String,
    pub title: String,
    /// epoch ms
    pub start: f64,
    pub end: f64,
    pub all_day: bool,
    pub calendar: String,
    pub location: Option<String>,
}

pub fn access() -> Access {
    // SAFETY: método de classe sem efeitos colaterais
    let status = unsafe { EKEventStore::authorizationStatusForEntityType(EKEntityType::Event) };
    match status {
        EKAuthorizationStatus::FullAccess => Access::Granted,
        EKAuthorizationStatus::NotDetermined => Access::Undetermined,
        _ => Access::Denied,
    }
}

/// Pede acesso total aos eventos e espera a resposta. Bloqueia: chamar fora da main thread.
fn request_blocking() -> Access {
    let (tx, rx) = mpsc::channel::<bool>();
    let block = RcBlock::new(move |granted: Bool, _err: *mut NSError| {
        let _ = tx.send(granted.as_bool());
    });
    let store = unsafe { EKEventStore::new() };
    // SAFETY: o bloco vive até o fim da função e o EventKit o copia se precisar
    unsafe {
        let ptr = &*block as *const _ as *mut _;
        if store.respondsToSelector(sel!(requestFullAccessToEventsWithCompletion:)) {
            store.requestFullAccessToEventsWithCompletion(ptr);
        } else {
            // macOS < 14
            #[allow(deprecated)]
            store.requestAccessToEntityType_completion(EKEntityType::Event, ptr);
        }
    }
    let _ = rx.recv_timeout(PROMPT_TIMEOUT);
    access()
}

/// Eventos entre `start_ms` e `end_ms` (epoch ms), por início. Vazio sem permissão.
pub fn events(start_ms: f64, end_ms: f64) -> Vec<CalEvent> {
    if access() != Access::Granted || end_ms <= start_ms {
        return vec![];
    }
    let to_date = |ms: f64| NSDate::dateWithTimeIntervalSince1970(ms / 1000.0);
    let (from, to) = (to_date(start_ms), to_date(end_ms));
    // SAFETY: store novo por chamada (vê as mudanças mais recentes do Calendário) e predicado criado por ele
    let found = unsafe {
        let store = EKEventStore::new();
        let predicate = store.predicateForEventsWithStartDate_endDate_calendars(&from, &to, None);
        store.eventsMatchingPredicate(&predicate)
    };
    let mut out: Vec<CalEvent> = found
        .iter()
        .map(|e| unsafe {
            CalEvent {
                id: e.eventIdentifier().map(|s| s.to_string()).unwrap_or_default(),
                title: e.title().to_string(),
                start: e.startDate().timeIntervalSince1970() * 1000.0,
                end: e.endDate().timeIntervalSince1970() * 1000.0,
                all_day: e.isAllDay(),
                calendar: e.calendar().map(|c| c.title().to_string()).unwrap_or_default(),
                location: e.location().map(|l| l.to_string()).filter(|l| !l.is_empty()),
            }
        })
        .collect();
    out.sort_by(|a, b| a.start.total_cmp(&b.start));
    out
}

#[tauri::command]
pub fn calendar_access() -> Access {
    access()
}

#[tauri::command]
pub async fn calendar_request_access(app: AppHandle) -> Access {
    // app Accessory (sem Dock): sem ativar, o diálogo de permissão pode nem aparecer
    crate::on_main(&app, |_| crate::macos::activate_app());
    tauri::async_runtime::spawn_blocking(request_blocking).await.unwrap_or(Access::Denied)
}

#[tauri::command]
pub async fn calendar_events(start: f64, end: f64) -> Vec<CalEvent> {
    tauri::async_runtime::spawn_blocking(move || events(start, end)).await.unwrap_or_default()
}
