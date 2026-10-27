import { useCallback, useEffect, useState } from "react";
import { native, subscribe, type CalendarAccess, type CalendarEvent, type Reminder } from "../../lib/native";

/** 6 semanas: o mês inteiro cabe sempre, com o tamanho da grade estável. */
export const GRID_DAYS = 42;
const REFRESH_MS = 60_000;

export const addDays = (d: Date, n: number) => new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);
export const startOfDay = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate());
export const sameDay = (a: Date, b: Date) => a.toDateString() === b.toDateString();

/** Primeiro dia da grade do mês (domingo da semana do dia 1). */
export function gridStart(year: number, month: number): Date {
  const first = new Date(year, month, 1);
  return addDays(first, -first.getDay());
}

/** Últimos eventos lidos e o estado da permissão: a aba abre já com eles e atualiza por baixo. */
let cached: { grid: number; access: CalendarAccess; events: CalendarEvent[] } | null = null;

/** Eventos do Calendário visíveis na grade e lembretes, só enquanto a aba está montada. */
export function useAgenda(grid: Date) {
  const gridMs = grid.getTime();
  const warm = cached?.grid === gridMs ? cached : null;
  const [access, setAccess] = useState<CalendarAccess | null>(warm?.access ?? null);
  const [events, setEvents] = useState<CalendarEvent[]>(warm?.events ?? []);
  const [reminders, setReminders] = useState<Reminder[]>([]);

  const load = useCallback(async () => {
    const a = await native.calendarAccess();
    setAccess(a);
    const fresh = a === "granted" ? await native.calendarEvents(gridMs, addDays(new Date(gridMs), GRID_DAYS).getTime()) : [];
    if (a === "granted") setEvents(fresh);
    cached = { grid: gridMs, access: a, events: fresh };
  }, [gridMs]);

  useEffect(() => {
    load().catch((e) => console.error("calendário falhou:", e));
    const id = setInterval(() => load().catch(() => {}), REFRESH_MS);
    return () => clearInterval(id);
  }, [load]);

  useEffect(() => {
    native.remindersList().then(setReminders);
    return subscribe(native.onReminders(setReminders));
  }, []);

  const requestAccess = useCallback(() => {
    native
      .calendarRequestAccess()
      .then((a) => {
        setAccess(a);
        return a === "granted" ? load() : undefined;
      })
      .catch((e) => console.error("permissão do calendário falhou:", e));
  }, [load]);

  return { access, events, reminders, requestAccess };
}

export type AgendaItem =
  | { kind: "event"; key: string; at: number; title: string; allDay: boolean }
  | { kind: "reminder"; key: string; at: number; title: string; overdue: boolean; id: number };

/** Itens do `day`. Evento aparece em todo dia que atravessa; lembrete vencido fica em hoje até ser concluído. */
export function itemsOf(day: Date, events: CalendarEvent[], reminders: Reminder[], now: number): AgendaItem[] {
  const from = startOfDay(day).getTime();
  const to = addDays(day, 1).getTime();
  const isToday = sameDay(day, new Date(now));
  const items: AgendaItem[] = [
    ...events
      .filter((e) => e.start < to && e.end > from)
      .map((e): AgendaItem => ({
        kind: "event",
        key: `${e.id}@${e.start}`,
        at: Math.max(e.start, from),
        title: e.title,
        // dia todo, ou continuação de um evento que começou antes
        allDay: e.allDay || e.start < from,
      })),
    ...reminders
      .filter((r) => (r.at >= from && r.at < to) || (isToday && r.at < from))
      .map((r): AgendaItem => ({ kind: "reminder", key: `r${r.id}`, at: r.at, title: r.text, overdue: r.at < now, id: r.id })),
  ];
  // dia inteiro primeiro, depois por horário
  return items.sort((a, b) => Number(b.kind === "event" && b.allDay) - Number(a.kind === "event" && a.allDay) || a.at - b.at);
}
