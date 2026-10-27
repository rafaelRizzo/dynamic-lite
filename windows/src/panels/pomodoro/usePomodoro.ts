import { useCallback, useEffect, useState } from "react";
import { native, subscribe, type PomodoroPhase, type PomodoroState, type Settings } from "../../lib/native";

/** Pomodoro sincronizado com o Rust, que é quem conta o tempo e dispara o fim da fase. */
export function usePomodoro() {
  const [state, setState] = useState<PomodoroState | null>(null);
  useEffect(() => {
    native.pomodoroState().then(setState);
    return subscribe(native.onPomodoro(setState));
  }, []);

  // o evento do Rust confirma e atualiza todas as janelas
  const toggle = useCallback(() => {
    const run = state?.status === "running" ? native.pomodoroPause : native.pomodoroStart;
    run().then(setState);
  }, [state?.status]);
  const skip = useCallback(() => void native.pomodoroSkip().then(setState), []);
  const reset = useCallback(() => void native.pomodoroReset().then(setState), []);

  return { state, toggle, skip, reset };
}

export function phaseMs(phase: PomodoroPhase, s: Settings): number {
  const min = phase === "focus" ? s.focusMin : phase === "short" ? s.shortBreakMin : s.longBreakMin;
  return min * 60_000;
}

/** Tempo que falta na fase; parado, a fase inteira. */
export function remainingMs(state: PomodoroState, s: Settings, now: number): number {
  // nunca mais que a fase inteira: um `now` atrasado de um frame não pode mostrar 25:02
  if (state.status === "running") return Math.min(phaseMs(state.phase, s), Math.max(0, state.endsAt - now));
  if (state.status === "paused") return state.remainingMs;
  return phaseMs(state.phase, s);
}

export function formatClock(ms: number): string {
  const total = Math.ceil(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const ss = String(total % 60).padStart(2, "0");
  return h > 0 ? `${h}:${String(m).padStart(2, "0")}:${ss}` : `${m}:${ss}`;
}

/** Qual setting guarda a duração de cada fase. */
export const PHASE_KEY = { focus: "focusMin", short: "shortBreakMin", long: "longBreakMin" } as const;

/** Passo dos botões +/-: fino nos tempos curtos, grosso nos longos. */
export function stepFor(minutes: number): number {
  return minutes < 10 ? 1 : minutes < 60 ? 5 : minutes < 180 ? 15 : 30;
}

/** Atalhos de tempo, por fase: um clique em vez de dezenas no +. */
export const PRESETS: Record<PomodoroPhase, { minutes: number; label: string }[]> = {
  focus: [
    { minutes: 15, label: "15m" },
    { minutes: 25, label: "25m" },
    { minutes: 45, label: "45m" },
    { minutes: 60, label: "1h" },
    { minutes: 120, label: "2h" },
    { minutes: 240, label: "4h" },
  ],
  short: [
    { minutes: 3, label: "3m" },
    { minutes: 5, label: "5m" },
    { minutes: 10, label: "10m" },
    { minutes: 15, label: "15m" },
  ],
  long: [
    { minutes: 10, label: "10m" },
    { minutes: 15, label: "15m" },
    { minutes: 20, label: "20m" },
    { minutes: 30, label: "30m" },
    { minutes: 60, label: "1h" },
  ],
};

export const PHASE_LABEL: Record<PomodoroPhase, string> = { focus: "Foco", short: "Pausa curta", long: "Pausa longa" };
