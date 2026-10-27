import { motion } from "motion/react";
import { useCallback, useEffect, useState } from "react";
import { native, subscribe, type Alert } from "../lib/native";
import { CalendarIcon, TimerIcon } from "../panels/icons";
import type { Panel } from "./types";

const ALERT_MS = 8000;
const SIZE = { width: 340, height: 96 };

function Expanded({ alert, topInset, onDismiss }: { alert: Alert; topInset: number; onDismiss: () => void }) {
  const startNext = alert.kind === "pomodoro" && !alert.started;
  const reminderId = alert.kind === "reminder" ? alert.reminderId : null;
  return (
    <div className="al-expanded" style={{ paddingTop: topInset }}>
      <div className="al-head">
        <span className="al-badge">{alert.kind === "pomodoro" ? <TimerIcon /> : <CalendarIcon />}</span>
        <div className="al-meta">
          <div className="al-title">{alert.title}</div>
          {alert.body && <div className="al-body">{alert.body}</div>}
        </div>
      </div>
      <div className="al-actions">
        {reminderId !== null ? (
          <>
            <motion.button
              className="up-btn"
              onClick={() => {
                native.remindersSnooze(reminderId, 5);
                onDismiss();
              }}
              whileTap={{ scale: 0.94 }}
            >
              Adiar 5 min
            </motion.button>
            <motion.button
              className="up-btn up-btn-primary"
              onClick={() => {
                native.remindersDone(reminderId);
                onDismiss();
              }}
              whileTap={{ scale: 0.94 }}
            >
              Concluir
            </motion.button>
          </>
        ) : (
          <motion.button className="up-btn" onClick={onDismiss} whileTap={{ scale: 0.94 }}>
            {startNext ? "Depois" : "Ok"}
          </motion.button>
        )}
        {startNext && (
          <motion.button
            className="up-btn up-btn-primary"
            onClick={() => {
              native.pomodoroStart();
              onDismiss();
            }}
            whileTap={{ scale: 0.94 }}
          >
            Iniciar
          </motion.button>
        )}
      </div>
    </div>
  );
}

/**
 * Avisos disparados pelo Rust. O primeiro da fila abre a Island sozinha (painel forçado, fora da barra de abas)
 * e some em alguns segundos; com o mouse em cima, fica.
 */
export function useAlertPanel(hovered: boolean): Panel | null {
  const [queue, setQueue] = useState<Alert[]>([]);
  useEffect(() => subscribe(native.onAlert((a) => setQueue((q) => [...q, a]))), []);

  const current = queue[0];
  const dismiss = useCallback(() => setQueue((q) => q.slice(1)), []);
  useEffect(() => {
    if (!current || hovered) return;
    const id = setTimeout(dismiss, ALERT_MS);
    return () => clearTimeout(id);
  }, [current, hovered, dismiss]);

  if (!current) return null;
  return {
    id: "alert",
    label: current.title,
    icon: null,
    hidden: true,
    expanded: ({ topInset }) => <Expanded alert={current} topInset={topInset} onDismiss={dismiss} />,
    expandedSize: SIZE,
  };
}
