export const MIN_MINUTES = 1;
/** 24 horas */
export const MAX_MINUTES = 24 * 60;

/** "25", "25m", "25 min", "1h", "1h30", "1:30" -> minutos (1 min a 24 h); inválido ou fora do limite: null. */
export function parseDuration(text: string): number | null {
  const t = text.trim().toLowerCase().replace(/\s+/g, "");
  let minutes: number | null = null;
  let m: RegExpExecArray | null;
  if ((m = /^(\d+)(?:m|min|mins|minutos?)?$/.exec(t))) minutes = Number(m[1]);
  else if ((m = /^(\d+)(?:h|hs|horas?)(?:(\d{1,2})(?:m|min)?)?$/.exec(t))) minutes = Number(m[1]) * 60 + Number(m[2] ?? 0);
  else if ((m = /^(\d+):(\d{2})$/.exec(t))) minutes = Number(m[1]) * 60 + Number(m[2]);
  if (minutes === null || minutes < MIN_MINUTES || minutes > MAX_MINUTES) return null;
  return minutes;
}

/** 25 -> "25 min", 90 -> "1h30", 120 -> "2h". */
export function formatDuration(minutes: number): string {
  if (minutes < 60) return `${minutes} min`;
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  return m === 0 ? `${h}h` : `${h}h${String(m).padStart(2, "0")}`;
}

export const clampMinutes = (m: number) => Math.min(MAX_MINUTES, Math.max(MIN_MINUTES, Math.round(m)));
