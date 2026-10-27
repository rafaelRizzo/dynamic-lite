//! Scheduler: Pomodoro (e, mais tarde, lembretes). Fonte da verdade no Rust, persistida em JSON,
//! pra sobreviver a reload do webview e a reinício do app.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::pomodoro::{Phase, Pomodoro, Status};
use crate::settings::SharedSettings;
use crate::sound;

pub const POMODORO_EVENT: &str = "pomodoro://state";
pub const REMINDERS_EVENT: &str = "reminders://changed";
pub const ALERT_EVENT: &str = "alert://fired";
const FILE: &str = "scheduler.json";
/// Teto de espera entre verificações: o relógio monotônico pára com o Mac dormindo.
const MAX_WAIT: Duration = Duration::from_secs(5);
/// De quanto em quanto tempo olhar o Calendário atrás de eventos prestes a começar.
const CALENDAR_POLL_MS: u64 = 30_000;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: u64,
    pub text: String,
    /// epoch ms
    pub at: u64,
    /// o aviso já foi dado (continua na lista até ser concluído)
    pub fired: bool,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Data {
    pomodoro: Pomodoro,
    reminders: Vec<Reminder>,
    next_id: u64,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AlertKind {
    Pomodoro,
    Reminder,
    Event,
}

/// Aviso que a Island mostra sozinha.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Alert {
    pub kind: AlertKind,
    pub title: String,
    pub body: Option<String>,
    /// Pomodoro: a próxima fase já começou (auto-iniciar)
    pub started: bool,
    /// Lembrete: pra adiar ou concluir
    pub reminder_id: Option<u64>,
}

pub struct Scheduler {
    data: Mutex<Data>,
    waker: Mutex<Sender<()>>,
    /// eventos do Calendário já avisados (`id@início`), só em memória
    alerted: Mutex<HashSet<String>>,
    last_calendar_check: Mutex<u64>,
}

pub type SharedScheduler = Arc<Scheduler>;

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|dir| dir.join(FILE))
}

fn save(app: &AppHandle, data: &Data) {
    let Some(p) = path(app) else { return };
    if let Some(dir) = p.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Ok(json) = serde_json::to_string_pretty(data) {
        let _ = fs::write(p, json);
    }
}

fn phase_label(phase: Phase) -> &'static str {
    match phase {
        Phase::Focus => "foco",
        Phase::Short => "pausa curta",
        Phase::Long => "pausa longa",
    }
}

fn pomodoro_alert(done: Phase, next: Phase, started: bool, next_min: u32) -> Alert {
    let title = match done {
        Phase::Focus => "Foco concluído",
        Phase::Short | Phase::Long => "Pausa acabou",
    };
    let body = if started {
        format!("Começou: {} de {next_min} min", phase_label(next))
    } else {
        format!("Próximo: {} de {next_min} min", phase_label(next))
    };
    Alert { kind: AlertKind::Pomodoro, title: title.into(), body: Some(body), started, reminder_id: None }
}

impl Scheduler {
    pub fn new(app: &AppHandle, waker: Sender<()>) -> SharedScheduler {
        let data = path(app)
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|raw| serde_json::from_str::<Data>(&raw).ok())
            .unwrap_or_default();
        Arc::new(Self {
            data: Mutex::new(data),
            waker: Mutex::new(waker),
            alerted: Mutex::default(),
            last_calendar_check: Mutex::default(),
        })
    }

    fn wake(&self) {
        let _ = self.waker.lock().unwrap().send(());
    }
}

fn settings_of(app: &AppHandle) -> crate::settings::Settings {
    app.state::<SharedSettings>().lock().unwrap().clone()
}

/// Muda o Pomodoro, persiste e avisa as janelas.
fn mutate(app: &AppHandle, sched: &Scheduler, f: impl FnOnce(&mut Pomodoro)) -> Pomodoro {
    let mut data = sched.data.lock().unwrap();
    f(&mut data.pomodoro);
    save(app, &data);
    let state = data.pomodoro.clone();
    drop(data);
    let _ = app.emit(POMODORO_EVENT, &state);
    sched.wake();
    state
}

#[tauri::command]
pub fn pomodoro_state(sched: State<'_, SharedScheduler>) -> Pomodoro {
    sched.data.lock().unwrap().pomodoro.clone()
}

#[tauri::command]
pub fn pomodoro_start(app: AppHandle, sched: State<'_, SharedScheduler>) -> Pomodoro {
    let d = settings_of(&app).durations();
    mutate(&app, &sched, |p| p.start(now_ms(), &d))
}

#[tauri::command]
pub fn pomodoro_pause(app: AppHandle, sched: State<'_, SharedScheduler>) -> Pomodoro {
    mutate(&app, &sched, |p| p.pause(now_ms()))
}

#[tauri::command]
pub fn pomodoro_reset(app: AppHandle, sched: State<'_, SharedScheduler>) -> Pomodoro {
    mutate(&app, &sched, Pomodoro::reset)
}

/// Pula a fase atual, sem som.
#[tauri::command]
pub fn pomodoro_skip(app: AppHandle, sched: State<'_, SharedScheduler>) -> Pomodoro {
    let s = settings_of(&app);
    let d = s.durations();
    mutate(&app, &sched, |p| {
        p.advance(now_ms(), &d, s.auto_start_next);
    })
}

/// Muda os lembretes, persiste e avisa as janelas.
fn mutate_reminders(app: &AppHandle, sched: &Scheduler, f: impl FnOnce(&mut Data)) -> Vec<Reminder> {
    let mut data = sched.data.lock().unwrap();
    f(&mut data);
    data.reminders.sort_by_key(|r| r.at);
    save(app, &data);
    let list = data.reminders.clone();
    drop(data);
    let _ = app.emit(REMINDERS_EVENT, &list);
    sched.wake();
    list
}

#[tauri::command]
pub fn reminders_list(sched: State<'_, SharedScheduler>) -> Vec<Reminder> {
    sched.data.lock().unwrap().reminders.clone()
}

#[tauri::command]
pub fn reminders_add(app: AppHandle, text: String, at: u64, sched: State<'_, SharedScheduler>) -> Result<Vec<Reminder>, String> {
    let text = text.trim().chars().take(120).collect::<String>();
    if text.is_empty() {
        return Err("texto vazio".into());
    }
    Ok(mutate_reminders(&app, &sched, |d| {
        d.next_id += 1;
        let id = d.next_id;
        d.reminders.push(Reminder { id, text, at, fired: false });
    }))
}

/// Adia o lembrete: `minutes` a partir de agora.
#[tauri::command]
pub fn reminders_snooze(app: AppHandle, id: u64, minutes: u64, sched: State<'_, SharedScheduler>) -> Vec<Reminder> {
    mutate_reminders(&app, &sched, |d| {
        if let Some(r) = d.reminders.iter_mut().find(|r| r.id == id) {
            r.at = now_ms() + minutes.clamp(1, 24 * 60) * 60_000;
            r.fired = false;
        }
    })
}

/// Concluir e apagar são a mesma coisa: o lembrete sai da lista.
#[tauri::command]
pub fn reminders_done(app: AppHandle, id: u64, sched: State<'_, SharedScheduler>) -> Vec<Reminder> {
    mutate_reminders(&app, &sched, |d| d.reminders.retain(|r| r.id != id))
}

/// Avisa dos eventos do Calendário que começam em até `lead_min`. Só consulta de tempos em tempos.
fn due_event_alerts(sched: &Scheduler, s: &crate::settings::Settings, now: u64) -> Vec<Alert> {
    if !s.show_agenda || s.calendar_lead_min == 0 {
        return vec![];
    }
    {
        let mut last = sched.last_calendar_check.lock().unwrap();
        if now.saturating_sub(*last) < CALENDAR_POLL_MS {
            return vec![];
        }
        *last = now;
    }
    let lead_ms = u64::from(s.calendar_lead_min) * 60_000;
    let events = crate::calendar::events(now as f64, (now + lead_ms) as f64);
    let mut alerted = sched.alerted.lock().unwrap();
    events
        .into_iter()
        .filter(|e| !e.all_day && alerted.insert(format!("{}@{}", e.id, e.start as u64)))
        .map(|e| {
            let mins = ((e.start as u64).saturating_sub(now) + 30_000) / 60_000;
            let when = if mins == 0 { "Começa agora".to_string() } else { format!("Começa em {mins} min") };
            let body = match e.location {
                Some(place) => format!("{when} · {place}"),
                None => when,
            };
            Alert { kind: AlertKind::Event, title: e.title, body: Some(body), started: false, reminder_id: None }
        })
        .collect()
}

/// Verifica o que venceu: toca o SFX e avisa a Island.
fn tick(app: &AppHandle, sched: &Scheduler) {
    let now = now_ms();
    let s = settings_of(app);
    let d = s.durations();
    let mut alerts = vec![];
    {
        let mut data = sched.data.lock().unwrap();
        if data.pomodoro.is_due(now) {
            let done = data.pomodoro.advance(now, &d, s.auto_start_next);
            let next = data.pomodoro.phase;
            let next_min = (d.of(next) / 60_000) as u32;
            alerts.push(pomodoro_alert(done, next, data.pomodoro.status == Status::Running, next_min));
            save(app, &data);
            let _ = app.emit(POMODORO_EVENT, &data.pomodoro);
        }

        let mut reminders_changed = false;
        for r in data.reminders.iter_mut().filter(|r| !r.fired && r.at <= now) {
            r.fired = true;
            reminders_changed = true;
            alerts.push(Alert {
                kind: AlertKind::Reminder,
                title: r.text.clone(),
                body: Some("Lembrete".into()),
                started: false,
                reminder_id: Some(r.id),
            });
        }
        if reminders_changed {
            save(app, &data);
            let _ = app.emit(REMINDERS_EVENT, &data.reminders);
        }
    }
    alerts.extend(due_event_alerts(sched, &s, now));
    for alert in alerts {
        sound::play(&s.sfx, s.sfx_volume);
        let _ = app.emit(ALERT_EVENT, &alert);
    }
}

/// Próximo vencimento, pra dormir só até lá (com teto).
fn wait_time(sched: &Scheduler) -> Duration {
    let data = sched.data.lock().unwrap();
    let p = &data.pomodoro;
    let mut wait = MAX_WAIT;
    let now = now_ms();
    if p.status == Status::Running {
        wait = wait.min(Duration::from_millis(p.ends_at.saturating_sub(now)));
    }
    if let Some(r) = data.reminders.iter().filter(|r| !r.fired).min_by_key(|r| r.at) {
        wait = wait.min(Duration::from_millis(r.at.saturating_sub(now)));
    }
    wait
}

pub fn spawn(app: AppHandle, sched: SharedScheduler, wake: Receiver<()>) {
    thread::spawn(move || loop {
        tick(&app, &sched);
        match wake.recv_timeout(wait_time(&sched).max(Duration::from_millis(50))) {
            Ok(()) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
    });
}
