//! Uso do Claude Code lido dos transcritos locais (`~/.claude/projects/**/*.jsonl`), sem rede (ADR 0008).
//! Leitura incremental: cada arquivo guarda até onde já foi lido e só o que foi anexado é processado.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const HOUR_MS: u64 = 3_600_000;
const BLOCK_MS: u64 = 5 * HOUR_MS;
/// Histórico considerado: o bastante pra achar onde o bloco de 5h atual começou.
const WINDOW: Duration = Duration::from_secs(48 * 3600);
/// JSON do statusline do Claude Code, gravado pelo `statusLine` do usuário (`cat > ~/.claude/dynano-usage.json`).
const STATUS_FILE: &str = ".claude/dynano-usage.json";

#[derive(Clone, Copy)]
struct Entry {
    ts: u64,
    input: u64,
    output: u64,
    cache_write: u64,
    cache_read: u64,
    cost: f64,
}

#[derive(Clone, Copy, Default, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub input: u64,
    pub output: u64,
    pub cache_write: u64,
    pub cache_read: u64,
    /// USD, estimado pela tabela de preços embutida
    pub cost: f64,
    pub messages: u32,
}

impl Totals {
    fn add(&mut self, e: &Entry) {
        self.input += e.input;
        self.output += e.output;
        self.cache_write += e.cache_write;
        self.cache_read += e.cache_read;
        self.cost += e.cost;
        self.messages += 1;
    }
}

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    /// epoch ms
    pub start: u64,
    pub end: u64,
    pub totals: Totals,
}

/// Um limite do plano: quanto já foi usado e quando reinicia.
#[derive(Clone, Copy, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Limit {
    /// 0-100
    pub used: f64,
    /// epoch ms
    pub resets_at: u64,
}

#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    pub five_hour: Option<Limit>,
    pub seven_day: Option<Limit>,
    /// quando o dado foi obtido (epoch ms): consulta ao vivo = agora; arquivo do statusline = quando foi gravado
    pub updated_at: u64,
    /// veio da consulta direta à conta (e não do arquivo do statusline)
    pub live: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub today: Totals,
    /// limites reais do plano; `None` se nem a consulta à conta nem o arquivo do statusline deram dado
    pub limits: Option<Limits>,
    /// por que a consulta à conta falhou (o arquivo do statusline cobre): "keychain", "expired" ou "network"
    pub live_error: Option<&'static str>,
    /// bloco de 5h em andamento, se houve uso nas últimas 5h
    pub block: Option<Block>,
}

// ---------- preço ----------

/// USD por milhão de tokens: (entrada, saída). Estimativa por família do modelo; escrita de cache = 1,25x a
/// entrada e leitura = 0,1x.
fn rates(model: &str) -> (f64, f64) {
    let m = model.to_ascii_lowercase();
    if m.contains("haiku") {
        (1.0, 5.0)
    } else if m.contains("opus") {
        // Opus 4 e 4.1 custavam 3x; os atuais não
        if m.contains("opus-4-1") || m.contains("opus-4-2025") || m.contains("claude-3-opus") {
            (15.0, 75.0)
        } else {
            (5.0, 25.0)
        }
    } else {
        (3.0, 15.0)
    }
}

fn cost_of(model: &str, input: u64, output: u64, cache_write: u64, cache_read: u64) -> f64 {
    let (rin, rout) = rates(model);
    let per = |tokens: u64, rate: f64| tokens as f64 * rate / 1_000_000.0;
    per(input, rin) + per(output, rout) + per(cache_write, rin * 1.25) + per(cache_read, rin * 0.1)
}

// ---------- tempo ----------

/// "2026-10-11T20:01:02.123Z" ou "...+00:00" / "...-03:00" -> epoch ms. Sem dependência de data.
fn parse_ts(s: &str) -> Option<u64> {
    let b = s.as_bytes();
    if b.len() < 19 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' {
        return None;
    }
    let n = |r: std::ops::Range<usize>| s.get(r)?.parse::<i64>().ok();
    let (y, mo, d) = (n(0..4)?, n(5..7)?, n(8..10)?);
    let (h, mi, sec) = (n(11..13)?, n(14..16)?, n(17..19)?);
    let mut pos = 19;
    let mut ms = 0;
    if b.get(pos) == Some(&b'.') {
        let digits = b[pos + 1..].iter().take_while(|c| c.is_ascii_digit()).count();
        ms = format!("{:0<3}", &s[pos + 1..pos + 1 + digits.min(3)]).parse::<i64>().ok()?;
        pos += 1 + digits;
    }
    // fuso: "Z" ou ±HH:MM (padrão UTC)
    let offset_min = match b.get(pos) {
        Some(&sign @ (b'+' | b'-')) => {
            let off = n(pos + 1..pos + 3)? * 60 + n(pos + 4..pos + 6)?;
            if sign == b'-' { -off } else { off }
        }
        _ => 0,
    };
    // dias desde 1970 (algoritmo de Howard Hinnant)
    let y = if mo <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if mo > 2 { mo - 3 } else { mo + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let total = ((days * 24 + h) * 60 + mi - offset_min) * 60 + sec;
    u64::try_from(total * 1000 + ms).ok()
}

// ---------- limites do plano (statusline) ----------

#[derive(Deserialize)]
struct StatusFile {
    rate_limits: Option<RateLimits>,
}

#[derive(Deserialize)]
struct RateLimits {
    five_hour: Option<RawLimit>,
    seven_day: Option<RawLimit>,
}

#[derive(Deserialize)]
struct RawLimit {
    used_percentage: Option<f64>,
    /// epoch segundos
    resets_at: Option<f64>,
}

fn limit_of(raw: Option<RawLimit>) -> Option<Limit> {
    let raw = raw?;
    Some(Limit { used: raw.used_percentage?.clamp(0.0, 100.0), resets_at: (raw.resets_at? * 1000.0) as u64 })
}

fn parse_limits(json: &str, updated_at: u64) -> Option<Limits> {
    let limits = serde_json::from_str::<StatusFile>(json).ok()?.rate_limits?;
    let (five_hour, seven_day) = (limit_of(limits.five_hour), limit_of(limits.seven_day));
    (five_hour.is_some() || seven_day.is_some()).then_some(Limits { five_hour, seven_day, updated_at, live: false })
}

// ---------- limites ao vivo (consulta à conta) ----------

const KEYCHAIN_ITEM: &str = "Claude Code-credentials";
const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
/// Entre consultas à API (a aba atualiza a cada 30s, mas o limite não muda tão rápido).
const LIVE_TTL: Duration = Duration::from_secs(120);
/// Depois de uma falha (Keychain negado, sem rede...), espera antes de tentar de novo.
const LIVE_BACKOFF: Duration = Duration::from_secs(300);

#[derive(Deserialize)]
struct Credentials {
    #[serde(rename = "claudeAiOauth")]
    oauth: Option<OAuth>,
}

#[derive(Deserialize)]
struct OAuth {
    #[serde(rename = "accessToken")]
    access_token: String,
    /// epoch ms
    #[serde(rename = "expiresAt")]
    expires_at: Option<u64>,
}

#[derive(Deserialize)]
struct ApiUsage {
    five_hour: Option<ApiLimit>,
    seven_day: Option<ApiLimit>,
}

#[derive(Deserialize)]
struct ApiLimit {
    /// 0-100
    utilization: Option<f64>,
    /// ISO 8601; `null` quando não há janela aberta
    resets_at: Option<String>,
}

fn api_limit(raw: Option<ApiLimit>) -> Option<Limit> {
    let raw = raw?;
    Some(Limit {
        used: raw.utilization?.clamp(0.0, 100.0),
        resets_at: raw.resets_at.as_deref().and_then(parse_ts).unwrap_or(0),
    })
}

fn parse_api_usage(json: &str, now: u64) -> Option<Limits> {
    let api: ApiUsage = serde_json::from_str(json).ok()?;
    let (five_hour, seven_day) = (api_limit(api.five_hour), api_limit(api.seven_day));
    (five_hour.is_some() || seven_day.is_some()).then_some(Limits { five_hour, seven_day, updated_at: now, live: true })
}

/// Token de acesso do Claude Code, do Keychain. Só em memória: nunca é salvo nem logado.
fn read_token(now: u64) -> Result<String, &'static str> {
    let out = Command::new("/usr/bin/security")
        .args(["find-generic-password", "-s", KEYCHAIN_ITEM, "-w"])
        .stderr(Stdio::null())
        .output()
        .map_err(|_| "keychain")?;
    if !out.status.success() {
        return Err("keychain");
    }
    let creds: Credentials = serde_json::from_slice(&out.stdout).map_err(|_| "keychain")?;
    let oauth = creds.oauth.ok_or("keychain")?;
    // o Claude Code renova o token quando roda; vencido, o arquivo do statusline cobre
    if oauth.expires_at.is_some_and(|t| t <= now + 30_000) {
        return Err("expired");
    }
    let ok = !oauth.access_token.is_empty() && oauth.access_token.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
    ok.then_some(oauth.access_token).ok_or("keychain")
}

/// GET na API de uso via `curl` (já vem no macOS). O token vai pela entrada padrão (`-K -`), não pelos argumentos,
/// que qualquer processo do usuário consegue ver.
fn http_usage(token: &str) -> Result<String, &'static str> {
    let mut child = Command::new("/usr/bin/curl")
        .args(["-sS", "--max-time", "10", "-w", "\n%{http_code}", "-K", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "network")?;
    let config = format!(
        "url = \"{USAGE_URL}\"\nheader = \"Authorization: Bearer {token}\"\nheader = \"anthropic-beta: oauth-2025-04-20\"\nheader = \"Accept: application/json\"\n"
    );
    child.stdin.take().ok_or("network")?.write_all(config.as_bytes()).map_err(|_| "network")?;
    let out = child.wait_with_output().map_err(|_| "network")?;
    let text = String::from_utf8_lossy(&out.stdout);
    let (body, status) = text.rsplit_once('\n').ok_or("network")?;
    if status.trim() == "200" { Ok(body.to_string()) } else { Err("network") }
}

type LiveResult = Result<Limits, &'static str>;

/// Limites ao vivo, com cache: 2 min no sucesso, 5 min na falha (não insiste num Keychain negado).
fn live_limits(now: u64) -> LiveResult {
    static CACHE: OnceLock<Mutex<Option<(Instant, LiveResult)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Mutex::default);
    let mut cached = cache.lock().unwrap();
    if let Some((at, result)) = cached.as_ref() {
        let ttl = if result.is_ok() { LIVE_TTL } else { LIVE_BACKOFF };
        if at.elapsed() < ttl {
            return result.clone_result();
        }
    }
    let result = read_token(now)
        .and_then(|token| http_usage(&token))
        .and_then(|body| parse_api_usage(&body, now).ok_or("network"));
    *cached = Some((Instant::now(), result.clone_result()));
    result
}

trait CloneResult {
    fn clone_result(&self) -> Self;
}

impl CloneResult for LiveResult {
    fn clone_result(&self) -> Self {
        match self {
            Ok(l) => Ok(Limits { five_hour: l.five_hour, seven_day: l.seven_day, updated_at: l.updated_at, live: l.live }),
            Err(e) => Err(e),
        }
    }
}

fn read_limits(home: &std::ffi::OsStr) -> Option<Limits> {
    let path = Path::new(home).join(STATUS_FILE);
    let updated = fs::metadata(&path).ok()?.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    parse_limits(&fs::read_to_string(path).ok()?, updated.as_millis() as u64)
}

// ---------- blocos de 5h ----------

/// Bloco de 5h em andamento: começa na hora cheia (UTC) do primeiro uso e dura 5h; um uso depois do fim, ou
/// depois de 5h de pausa, abre outro. Só conta se ainda não terminou. `entries` ordenadas por `ts`.
fn active_block(entries: &[Entry], now: u64) -> Option<Block> {
    let mut start = 0;
    let mut first = 0;
    let mut last = 0;
    for (i, e) in entries.iter().enumerate() {
        let continues = i > 0 && e.ts < start + BLOCK_MS && e.ts - last < BLOCK_MS;
        if !continues {
            start = e.ts - e.ts % HOUR_MS;
            first = i;
        }
        last = e.ts;
    }
    if entries.is_empty() || now >= start + BLOCK_MS || now.saturating_sub(last) >= BLOCK_MS {
        return None;
    }
    let mut totals = Totals::default();
    entries[first..].iter().for_each(|e| totals.add(e));
    Some(Block { start, end: start + BLOCK_MS, totals })
}

// ---------- leitura incremental ----------

#[derive(Deserialize)]
struct Line {
    timestamp: Option<String>,
    #[serde(rename = "requestId")]
    request_id: Option<String>,
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    id: Option<String>,
    model: Option<String>,
    usage: Option<RawUsage>,
}

#[derive(Deserialize, Default)]
struct RawUsage {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
}

#[derive(Default)]
struct FileState {
    /// bytes já consumidos (sempre até o fim de uma linha completa)
    offset: u64,
    entries: Vec<Entry>,
}

#[derive(Default)]
struct State {
    files: HashMap<PathBuf, FileState>,
    /// `message.id:requestId`: o mesmo turno aparece em várias linhas (uma por bloco de conteúdo)
    seen: HashSet<String>,
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(Mutex::default)
}

fn parse_line(raw: &str, seen: &mut HashSet<String>) -> Option<Entry> {
    // a maioria das linhas (prompts, ferramentas) não tem uso: nem parseia
    if !raw.contains("\"usage\"") {
        return None;
    }
    let line: Line = serde_json::from_str(raw).ok()?;
    let msg = line.message?;
    let usage = msg.usage?;
    let model = msg.model.unwrap_or_default();
    if model.starts_with('<') {
        return None; // "<synthetic>"
    }
    if let Some(id) = msg.id {
        if !seen.insert(format!("{id}:{}", line.request_id.unwrap_or_default())) {
            return None;
        }
    }
    let (input, output) = (usage.input_tokens.unwrap_or(0), usage.output_tokens.unwrap_or(0));
    let (cache_write, cache_read) = (usage.cache_creation_input_tokens.unwrap_or(0), usage.cache_read_input_tokens.unwrap_or(0));
    Some(Entry {
        ts: parse_ts(&line.timestamp?)?,
        input,
        output,
        cache_write,
        cache_read,
        cost: cost_of(&model, input, output, cache_write, cache_read),
    })
}

fn collect(dir: &Path, cutoff: SystemTime, out: &mut Vec<PathBuf>) {
    let Ok(read) = fs::read_dir(dir) else { return };
    for entry in read.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            collect(&path, cutoff, out);
        } else if path.extension().is_some_and(|e| e == "jsonl") && meta.modified().is_ok_and(|m| m >= cutoff) {
            out.push(path);
        }
    }
}

/// Lê o que foi anexado a `path` desde a última vez.
fn ingest(path: &Path, st: &mut State) {
    let Ok(mut file) = File::open(path) else { return };
    let Ok(len) = file.metadata().map(|m| m.len()) else { return };
    let fs_state = st.files.entry(path.to_path_buf()).or_default();
    if len < fs_state.offset {
        *fs_state = FileState::default(); // arquivo recomeçou
    }
    if len == fs_state.offset || file.seek(SeekFrom::Start(fs_state.offset)).is_err() {
        return;
    }
    let mut chunk = Vec::with_capacity((len - fs_state.offset) as usize);
    if file.take(len - fs_state.offset).read_to_end(&mut chunk).is_err() {
        return;
    }
    // a última linha pode estar sendo escrita: só consome até o último "\n"
    let Some(end) = chunk.iter().rposition(|&b| b == b'\n') else { return };
    fs_state.offset += end as u64 + 1;
    let text = String::from_utf8_lossy(&chunk[..end]);
    let (files, seen) = (&mut st.files, &mut st.seen);
    let entries = &mut files.get_mut(path).expect("inserido acima").entries;
    entries.extend(text.lines().filter_map(|l| parse_line(l, seen)));
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Totais de hoje (`day_start` = meia-noite local, epoch ms) e o bloco de 5h em andamento.
pub fn read(day_start: u64, live: bool) -> Usage {
    let now = now_ms();
    let Some(home) = std::env::var_os("HOME") else { return Usage { today: Totals::default(), limits: None, live_error: None, block: None } };
    let mut paths = vec![];
    collect(&Path::new(&home).join(".claude/projects"), SystemTime::now() - WINDOW, &mut paths);

    let mut st = state().lock().unwrap();
    // arquivos que saíram da janela liberam memória
    st.files.retain(|p, _| paths.contains(p));
    for p in &paths {
        ingest(p, &mut st);
    }
    let horizon = now.saturating_sub(WINDOW.as_millis() as u64);
    let mut entries: Vec<Entry> = st.files.values().flat_map(|f| f.entries.iter().copied()).filter(|e| e.ts >= horizon).collect();
    entries.sort_by_key(|e| e.ts);

    let mut today = Totals::default();
    entries.iter().filter(|e| e.ts >= day_start).for_each(|e| today.add(e));
    // ao vivo primeiro; se falhar (ou estiver desligado), o arquivo do statusline cobre
    let (limits, live_error) = match live.then(|| live_limits(now)) {
        Some(Ok(l)) => (Some(l), None),
        Some(Err(e)) => (read_limits(&home), Some(e)),
        None => (read_limits(&home), None),
    };
    Usage { today, limits, live_error, block: active_block(&entries, now) }
}

#[tauri::command]
pub async fn claude_usage(day_start: u64, settings: tauri::State<'_, crate::settings::SharedSettings>) -> Result<Usage, String> {
    let live = settings.lock().unwrap().claude_live_limits;
    tauri::async_runtime::spawn_blocking(move || read(day_start, live)).await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(ts: u64) -> Entry {
        Entry { ts, input: 1, output: 2, cache_write: 0, cache_read: 0, cost: 0.5 }
    }

    /// Roda com `cargo test real_usage -- --ignored --nocapture` pra ver o uso real desta máquina.
    #[test]
    #[ignore]
    fn real_usage_smoke() {
        let day_start = now_ms() - now_ms() % 86_400_000;
        let t = std::time::Instant::now();
        let u = read(day_start, std::env::var_os("LIVE_SMOKE").is_some());
        println!("1a leitura {:?}: limites {:?} | erro ao vivo {:?}", t.elapsed(), u.limits, u.live_error);
        let t = std::time::Instant::now();
        read(day_start, false);
        println!("2a leitura (incremental) {:?}", t.elapsed());
    }

    #[test]
    fn reads_plan_limits_from_the_statusline_json() {
        let json = r#"{"model":{},"rate_limits":{"seven_day":{"used_percentage":57,"resets_at":1791968400}}}"#;
        let l = parse_limits(json, 5).unwrap();
        assert_eq!(l.seven_day, Some(Limit { used: 57.0, resets_at: 1_791_968_400_000 }));
        assert_eq!((l.five_hour, l.updated_at), (None, 5));
        assert!(parse_limits(r#"{"model":{}}"#, 0).is_none());
        assert!(parse_limits("lixo", 0).is_none());
    }

    #[test]
    fn parses_timezone_offsets() {
        assert_eq!(parse_ts("2026-10-11T20:01:02.000000+00:00"), Some(1_791_748_862_000));
        // -03:00: três horas a mais em UTC
        assert_eq!(parse_ts("2026-10-11T17:01:02-03:00"), Some(1_791_748_862_000));
    }

    #[test]
    fn reads_the_account_usage_response() {
        let json = r#"{"five_hour":{"utilization":12.5,"resets_at":"2026-10-11T22:00:00.000000+00:00"},"seven_day":{"utilization":57.0,"resets_at":null},"seven_day_opus":null}"#;
        let l = parse_api_usage(json, 9).unwrap();
        assert_eq!(l.five_hour, Some(Limit { used: 12.5, resets_at: 1_791_756_000_000 }));
        assert_eq!(l.seven_day, Some(Limit { used: 57.0, resets_at: 0 }));
        assert!(l.live);
        assert!(parse_api_usage("{}", 0).is_none());
    }

    #[test]
    fn parses_iso_timestamps() {
        assert_eq!(parse_ts("1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(parse_ts("2026-10-11T20:01:02.5Z"), Some(1_791_748_862_500));
        assert_eq!(parse_ts("2024-02-29T12:00:00Z"), Some(1_709_208_000_000));
        assert_eq!(parse_ts("lixo"), None);
    }

    #[test]
    fn block_starts_at_the_hour_and_lasts_five_hours() {
        let t0 = 10 * HOUR_MS + 25 * 60_000; // 10:25
        let entries = [entry(t0), entry(t0 + HOUR_MS)];
        let b = active_block(&entries, t0 + 2 * HOUR_MS).unwrap();
        assert_eq!((b.start, b.end), (10 * HOUR_MS, 15 * HOUR_MS));
        assert_eq!((b.totals.messages, b.totals.output), (2, 4));
    }

    #[test]
    fn block_is_gone_after_it_ends_or_after_a_long_pause() {
        let t0 = 10 * HOUR_MS;
        assert!(active_block(&[entry(t0)], t0 + BLOCK_MS).is_none());
        assert!(active_block(&[], t0).is_none());
    }

    #[test]
    fn use_after_the_end_opens_a_new_block() {
        let t0 = 10 * HOUR_MS;
        let late = t0 + BLOCK_MS + 20 * 60_000;
        let b = active_block(&[entry(t0), entry(late)], late + 1000).unwrap();
        assert_eq!((b.start, b.totals.messages), (15 * HOUR_MS, 1));
    }

    #[test]
    fn prices_by_family_with_cache_multipliers() {
        // 1M entrada + 1M saída no Sonnet: 3 + 15
        assert!((cost_of("claude-sonnet-5-5", 1_000_000, 1_000_000, 0, 0) - 18.0).abs() < 1e-9);
        // leitura de cache custa 0,1x a entrada
        assert!((cost_of("claude-sonnet-5-5", 0, 0, 0, 1_000_000) - 0.3).abs() < 1e-9);
        assert!((cost_of("claude-opus-4-1-20250805", 1_000_000, 0, 0, 0) - 15.0).abs() < 1e-9);
    }

    #[test]
    fn same_turn_in_many_lines_counts_once() {
        let line = r#"{"timestamp":"2026-10-11T20:00:00.000Z","requestId":"r1","message":{"id":"m1","model":"claude-sonnet-5-5","usage":{"input_tokens":10,"output_tokens":5}}}"#;
        let mut seen = HashSet::new();
        assert!(parse_line(line, &mut seen).is_some());
        assert!(parse_line(line, &mut seen).is_none());
        assert!(parse_line(r#"{"type":"user","message":{"content":"oi"}}"#, &mut seen).is_none());
    }
}
