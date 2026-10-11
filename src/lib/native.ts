import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Notch = { width: number; height: number; hasNotch: boolean };

export type PlayerStatus = "closed" | "stopped" | "playing" | "paused";

export type Track = {
  id: string;
  name: string;
  artist: string;
  album: string;
  artworkUrl: string;
  /** segundos */
  duration: number;
};

export type PlayerState = {
  status: PlayerStatus;
  track: Track | null;
  /** segundos, no momento da leitura */
  position: number;
  /** 0-100 */
  volume: number;
  shuffle: boolean;
  /** `track` (repetir a faixa) é emulado pelo app; o Spotify só tem repetir liga/desliga */
  repeat: Repeat;
};

export type Repeat = "off" | "context" | "track";

export type Style = "black" | "translucent" | "glass" | "themed";

export type Settings = {
  style: Style;
  /** 0.6-1, só no Style Translucent */
  opacity: number;
  expandOn: "hover" | "click";
  hoverDelayMs: number;
  /** espera depois que o mouse sai antes de fechar; voltar nesse intervalo cancela */
  hoverExitDelayMs: number;
  hideIdleWithoutNotch: boolean;
  accent: "artwork" | "white";
  haptics: boolean;
  /** deslizar com 3 dedos na Island aberta troca de aba */
  swipeTabs: boolean;
  invertSwipe: boolean;
  showMenuBarIcon: boolean;
  /** "auto", "main" ou o nome da tela */
  display: string;
  allSpaces: boolean;
  hideInMissionControl: boolean;
  autoUpdate: boolean;
  showClaudeUsage: boolean;
  /** consulta o limite direto na conta (token do Keychain) em vez de só ler o arquivo do statusline */
  claudeLiveLimits: boolean;
  showAgenda: boolean;
  /** avisa N min antes de cada evento do Calendário; 0 desliga */
  calendarLeadMin: number;
  showPomodoro: boolean;
  /** minutos */
  focusMin: number;
  shortBreakMin: number;
  longBreakMin: number;
  /** pausa longa a cada N focos */
  longEvery: number;
  autoStartNext: boolean;
  /** nome de um som de /System/Library/Sounds */
  sfx: string;
  /** 0.1-1 */
  sfxVolume: number;
  showShortcuts: boolean;
  /** ids de `panels/shortcuts/catalog.ts`, na ordem da grade */
  shortcuts: string[];
};

export type PomodoroPhase = "focus" | "short" | "long";

export type PomodoroState = {
  status: "idle" | "running" | "paused";
  phase: PomodoroPhase;
  /** epoch ms em que a fase acaba (só `running`) */
  endsAt: number;
  /** ms que faltavam quando pausou (só `paused`) */
  remainingMs: number;
  /** focos concluídos desde a última pausa longa */
  cycle: number;
};

/** Aviso que a Island abre sozinha pra mostrar. */
export type Alert = {
  kind: "pomodoro" | "reminder" | "event";
  title: string;
  body: string | null;
  /** Pomodoro: a próxima fase já começou (auto-iniciar) */
  started: boolean;
  /** Lembrete: pra adiar ou concluir */
  reminderId: number | null;
};

export type Reminder = {
  id: number;
  text: string;
  /** epoch ms */
  at: number;
  /** o aviso já foi dado; fica na lista até ser concluído */
  fired: boolean;
};

export type CalendarAccess = "undetermined" | "granted" | "denied";

export type CalendarEvent = {
  /** recorrentes repetem o id: a chave única é `id` + `start` */
  id: string;
  title: string;
  /** epoch ms */
  start: number;
  end: number;
  allDay: boolean;
  calendar: string;
  location: string | null;
};

export type UsageTotals = {
  input: number;
  output: number;
  cacheWrite: number;
  cacheRead: number;
  /** USD estimado */
  cost: number;
  messages: number;
};

/** Limite do plano: quanto já foi usado e quando reinicia. */
export type UsageLimit = {
  /** 0-100 */
  used: number;
  /** epoch ms */
  resetsAt: number;
};

export type UsageLimits = {
  fiveHour: UsageLimit | null;
  sevenDay: UsageLimit | null;
  /** quando o dado foi obtido (epoch ms) */
  updatedAt: number;
  /** veio da consulta direta à conta, não do arquivo do statusline */
  live: boolean;
};

export type ClaudeUsage = {
  today: UsageTotals;
  /** limites reais do plano; null se nem a conta nem o arquivo do statusline deram dado */
  limits: UsageLimits | null;
  /** por que a consulta direta à conta falhou (o arquivo do statusline cobre) */
  liveError: "keychain" | "expired" | "network" | null;
  /** bloco de 5h em andamento; null se não houve uso nas últimas 5h */
  block: { start: number; end: number; totals: UsageTotals } | null;
};

export type UpdateInfo = { version: string; currentVersion: string; notes: string | null };

export type Platform = { glassSupported: boolean };

/** Forma da Island centrada no topo da janela, Ears incluídas. */
export type IslandShape = { width: number; height: number; radius: number; ear: number };

export type SpotifyAction = "playpause" | "next" | "previous";

const on =
  <T,>(event: string) =>
  (cb: (payload: T) => void): Promise<UnlistenFn> =>
    listen<T>(event, (e) => cb(e.payload));

/** Ponte com o backend Rust: comandos e eventos. */
export const native = {
  getNotch: () => invoke<Notch>("get_notch"),
  setIslandSize: (width: number, height: number) => invoke<void>("set_island_size", { width, height }),
  getPlatform: () => invoke<Platform>("get_platform"),
  listDisplays: () => invoke<string[]>("list_displays"),
  /** `target` null recolhe o vidro de volta pra `rest` (Compact/Idle) e esconde. */
  setGlass: (target: IslandShape | null, rest: IslandShape) => invoke<void>("set_glass", { target, rest }),
  haptic: () => invoke<void>("haptic"),
  /** só URLs `x-apple.systempreferences:` */
  openSystemSettings: (url: string) => invoke<void>("open_system_settings", { url }),
  showContextMenu: () => invoke<void>("show_context_menu"),
  getUpdate: () => invoke<UpdateInfo | null>("get_update"),
  checkUpdate: () => invoke<UpdateInfo | null>("check_update"),
  /** baixa, instala e reabre o app (não retorna em caso de sucesso) */
  installUpdate: () => invoke<void>("install_update"),
  getSettings: () => invoke<Settings>("get_settings"),
  setSettings: (value: Settings) => invoke<void>("set_settings", { value }),
  pomodoroState: () => invoke<PomodoroState>("pomodoro_state"),
  pomodoroStart: () => invoke<PomodoroState>("pomodoro_start"),
  pomodoroPause: () => invoke<PomodoroState>("pomodoro_pause"),
  pomodoroReset: () => invoke<PomodoroState>("pomodoro_reset"),
  /** pula a fase atual, sem som */
  pomodoroSkip: () => invoke<PomodoroState>("pomodoro_skip"),
  remindersList: () => invoke<Reminder[]>("reminders_list"),
  remindersAdd: (text: string, at: number) => invoke<Reminder[]>("reminders_add", { text, at }),
  remindersSnooze: (id: number, minutes: number) => invoke<Reminder[]>("reminders_snooze", { id, minutes }),
  /** concluir e apagar são a mesma coisa */
  remindersDone: (id: number) => invoke<Reminder[]>("reminders_done", { id }),
  calendarAccess: () => invoke<CalendarAccess>("calendar_access"),
  /** mostra o diálogo de permissão do macOS e espera a resposta */
  calendarRequestAccess: () => invoke<CalendarAccess>("calendar_request_access"),
  calendarEvents: (start: number, end: number) => invoke<CalendarEvent[]>("calendar_events", { start, end }),
  /** `dayStart`: meia-noite local, epoch ms */
  claudeUsage: (dayStart: number) => invoke<ClaudeUsage>("claude_usage", { dayStart }),
  listSounds: () => invoke<string[]>("list_sounds"),
  playSound: (name: string, volume: number) => invoke<void>("play_sound", { name, volume }),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => invoke<boolean>("set_autostart", { enabled }),
  spotifyState: () => invoke<PlayerState>("spotify_state"),
  spotifyControl: (action: SpotifyAction) => invoke<void>("spotify_control", { action }),
  spotifySeek: (position: number) => invoke<void>("spotify_seek", { position }),
  spotifyVolume: (volume: number) => invoke<void>("spotify_volume", { volume }),
  spotifyShuffle: (enabled: boolean) => invoke<void>("spotify_shuffle", { enabled }),
  spotifyRepeat: (mode: Repeat) => invoke<void>("spotify_repeat", { mode }),
  /** Traz o Spotify pra frente na faixa (`spotify:track:...`), sem mexer na reprodução. */
  spotifyOpen: (uri: string) => invoke<void>("spotify_open", { uri }),
  onNotch: on<Notch>("island://notch"),
  onHover: on<boolean>("island://hover"),
  /** 3 dedos no trackpad com o mouse na Island: 1 avança, -1 volta */
  onSwipe: on<number>("island://swipe"),
  onSpotify: on<PlayerState>("spotify://state"),
  /** Equalizer ao vivo: 4 bandas 0-1 (grave → agudo), ~30fps enquanto toca. Não chega sem permissão/macOS < 14.2. */
  onLevels: on<number[]>("spotify://levels"),
  onSettings: on<Settings>("settings://changed"),
  onPomodoro: on<PomodoroState>("pomodoro://state"),
  onReminders: on<Reminder[]>("reminders://changed"),
  onAlert: on<Alert>("alert://fired"),
  onUpdate: on<UpdateInfo | null>("update://available"),
  onUpdateInstalling: on<null>("update://installing"),
};

/** Assina um evento no useEffect sem vazar listener se o componente desmontar antes do `listen` resolver. */
export function subscribe(pending: Promise<UnlistenFn>): () => void {
  let unlisten: UnlistenFn | undefined;
  let disposed = false;
  pending.then((fn) => (disposed ? fn() : (unlisten = fn)));
  return () => {
    disposed = true;
    unlisten?.();
  };
}
