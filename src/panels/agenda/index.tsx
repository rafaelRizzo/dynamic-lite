import { useState } from "react";
import type { Panel } from "../../activities/types";
import { useNow } from "../../hooks/useNow";
import { native, type Settings } from "../../lib/native";
import { CalendarIcon } from "../icons";
import { parseReminder } from "./parse";
import { addDays, GRID_DAYS, gridStart, itemsOf, sameDay, startOfDay, useAgenda } from "./useAgenda";

const WIDTH = 440;
const HEIGHT = 172;
const PRIVACY_CALENDARS = "x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars";
const HINT = "20m ligar · 15:30 reunião · amanhã 9h pagar";
const WEEKDAYS = ["D", "S", "T", "Q", "Q", "S", "S"];

const time = (ms: number) => new Date(ms).toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });
const monthTitle = (y: number, m: number) => {
  const t = new Date(y, m, 1).toLocaleDateString("pt-BR", { month: "long", year: "numeric" }).replace(" de ", " ");
  return t.charAt(0).toUpperCase() + t.slice(1);
};
const dayTitle = (d: Date) => {
  const t = d.toLocaleDateString("pt-BR", { weekday: "long", day: "numeric", month: "short" }).replace(".", "");
  return t.charAt(0).toUpperCase() + t.slice(1);
};

const Chevron = ({ dir }: { dir: "left" | "right" }) => (
  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round">
    <path d={dir === "left" ? "m15 5-7 7 7 7" : "m9 5 7 7-7 7"} />
  </svg>
);

function Expanded({ topInset }: { topInset: number }) {
  const now = useNow(30_000);
  const today = startOfDay(new Date(now));
  const [view, setView] = useState({ year: today.getFullYear(), month: today.getMonth() });
  const [selected, setSelected] = useState(today);
  const [text, setText] = useState("");
  const [invalid, setInvalid] = useState(false);

  const grid = gridStart(view.year, view.month);
  const { access, events, reminders, requestAccess } = useAgenda(grid);
  const cells = Array.from({ length: GRID_DAYS }, (_, i) => addDays(grid, i));
  const items = itemsOf(selected, events, reminders, now);

  const goMonth = (delta: number) => {
    const d = new Date(view.year, view.month + delta, 1);
    setView({ year: d.getFullYear(), month: d.getMonth() });
    // volta pra hoje no mês atual, senão cai no dia 1
    setSelected(d.getFullYear() === today.getFullYear() && d.getMonth() === today.getMonth() ? today : d);
  };
  const goToday = () => {
    setView({ year: today.getFullYear(), month: today.getMonth() });
    setSelected(today);
  };

  const add = () => {
    if (!text.trim()) return;
    const parsed = parseReminder(text, new Date(), sameDay(selected, today) ? undefined : selected);
    if (!parsed) return setInvalid(true);
    native.remindersAdd(parsed.text, parsed.at).then(() => setText(""));
  };

  return (
    <div className="ag-expanded" style={{ paddingTop: topInset }}>
      <div className="ag-month">
        <div className="ag-month-head">
          <button className="ag-nav" onClick={() => goMonth(-1)} title="Mês anterior" aria-label="Mês anterior"><Chevron dir="left" /></button>
          <button className="ag-month-title" onClick={goToday} title="Ir pra hoje">{monthTitle(view.year, view.month)}</button>
          <button className="ag-nav" onClick={() => goMonth(1)} title="Próximo mês" aria-label="Próximo mês"><Chevron dir="right" /></button>
        </div>
        <div className="ag-grid">
          {WEEKDAYS.map((w, i) => (
            <span key={i} className="ag-wd">{w}</span>
          ))}
          {cells.map((d) => {
            const inMonth = d.getMonth() === view.month;
            const has = itemsOf(d, events, reminders, now).length > 0;
            return (
              <button
                key={d.getTime()}
                className="ag-cell"
                data-out={!inMonth}
                data-today={sameDay(d, today)}
                data-selected={sameDay(d, selected)}
                onClick={() => setSelected(d)}
              >
                {d.getDate()}
                <i data-on={has} />
              </button>
            );
          })}
        </div>
      </div>

      <div className="ag-day-col">
        <div className="ag-day-title">{dayTitle(selected)}</div>
        <ul className="ag-list">
          {items.map((it) => (
            <li key={it.key} className="ag-row" data-overdue={it.kind === "reminder" && it.overdue}>
              <span className="ag-time">{it.kind === "event" && it.allDay ? "dia todo" : time(it.at)}</span>
              <span className="ag-title" title={it.title}>{it.title}</span>
              {it.kind === "reminder" ? (
                <button className="ag-check" title="Concluir" onClick={() => native.remindersDone(it.id)}>✓</button>
              ) : (
                <span className="ag-dot" />
              )}
            </li>
          ))}
          {items.length === 0 && access === "granted" && <li className="ag-empty">Nada neste dia</li>}
          {access === "undetermined" && (
            <li className="ag-row">
              <button className="ag-cta" onClick={requestAccess}>Conectar ao Calendário</button>
            </li>
          )}
          {access === "denied" && (
            <li className="ag-row">
              <button className="ag-cta" onClick={() => native.openSystemSettings(PRIVACY_CALENDARS)}>
                Sem acesso: abrir Privacidade
              </button>
            </li>
          )}
        </ul>
        <input
          className="ag-input"
          data-invalid={invalid}
          value={text}
          placeholder={invalid ? `Não entendi. Tente: ${HINT}` : "Novo lembrete: 20m ligar"}
          title={HINT}
          maxLength={120}
          onChange={(e) => {
            setText(e.target.value);
            setInvalid(false);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") add();
            if (e.key === "Escape") e.currentTarget.blur();
          }}
        />
      </div>
    </div>
  );
}

/** Agenda: mês inteiro com eventos do Calendário do macOS e lembretes com hora. */
export function useAgendaPanel(settings: Settings): Panel | null {
  if (!settings.showAgenda) return null;
  return {
    id: "agenda",
    label: "Agenda",
    icon: <CalendarIcon />,
    expanded: ({ topInset }) => <Expanded topInset={topInset} />,
    expandedSize: { width: WIDTH, height: HEIGHT },
  };
}
