export type ParsedReminder = { at: number; text: string };

/** "20m ligar", "em 2h revisar", "15:30 reunião", "9h30 dentista", "amanhã 9h pagar". Sem texto ou hora válida: null. */
const MINUTES = /^(?:(?:em|daqui)\s+)?(\d+)\s*(?:m|min|mins|minutos?)\s+(.+)$/i;
const HOURS = /^(?:em|daqui)\s+(\d+)\s*(?:h|hs|horas?)\s+(.+)$/i;
const CLOCK = /^(amanh[ãa]\s+)?(?:[àa]s\s+)?(\d{1,2})(?::(\d{2})|h(\d{2})?)\s+(.+)$/i;

const MINUTE = 60_000;

/**
 * `day` é o dia selecionado na Agenda: hora do relógio vale pra ele (e "amanhã" soma um dia).
 * Hoje, uma hora que já passou vai pro dia seguinte.
 */
export function parseReminder(input: string, now: Date, day: Date = now): ParsedReminder | null {
  const raw = input.trim().replace(/\s+/g, " ");

  const minutes = MINUTES.exec(raw);
  if (minutes) return { at: now.getTime() + Number(minutes[1]) * MINUTE, text: minutes[2] };

  const hours = HOURS.exec(raw);
  if (hours) return { at: now.getTime() + Number(hours[1]) * 60 * MINUTE, text: hours[2] };

  const clock = CLOCK.exec(raw);
  if (!clock) return null;
  const hh = Number(clock[2]);
  const mm = Number(clock[3] ?? clock[4] ?? 0);
  if (hh > 23 || mm > 59) return null;
  const at = new Date(day.getFullYear(), day.getMonth(), day.getDate() + (clock[1] ? 1 : 0), hh, mm);
  const sameDayAsNow = at.toDateString() === now.toDateString();
  if (sameDayAsNow && at.getTime() <= now.getTime()) at.setDate(at.getDate() + 1);
  return { at: at.getTime(), text: clock[5] };
}
