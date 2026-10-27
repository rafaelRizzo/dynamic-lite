import { AnimatePresence, motion } from "motion/react";
import { useCallback, useEffect, useRef, useState } from "react";
import type { Activity, Panel } from "../../activities/types";
import { NextIcon, PauseIcon, PlayIcon } from "../../activities/now-playing/Icons";
import { useNow } from "../../hooks/useNow";
import { clampMinutes, parseDuration } from "../../lib/duration";
import type { PomodoroState, Settings } from "../../lib/native";
import { TimerIcon } from "../icons";
import { formatClock, phaseMs, PHASE_KEY, PHASE_LABEL, PRESETS, remainingMs, stepFor, usePomodoro } from "./usePomodoro";

const clockTime = (ms: number) => new Date(ms).toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" });

/**
 * Posição da fase atual na sequência do ciclo: foco, pausa curta, foco, pausa curta... e a pausa longa no fim
 * (com `longEvery` = 4: F S F S F S F L, 8 bolinhas). `cycle` já conta o foco que acabou de terminar.
 */
function cycleIndex(state: PomodoroState, longEvery: number): number {
  if (state.phase === "long") return longEvery * 2 - 1;
  return state.phase === "focus" ? state.cycle * 2 : state.cycle * 2 - 1;
}

/** Fase que vem depois da atual, igual à regra do Rust: a cada N focos, pausa longa. */
function nextPhase(state: PomodoroState, longEvery: number): PomodoroState["phase"] {
  if (state.phase !== "focus") return "focus";
  return state.cycle + 1 >= longEvery ? "long" : "short";
}

const WIDTH = 400;
const HEIGHT = 128;
const RING = 108;
const STROKE = 7;

/** Foco em verde, pausas em azul (as bolinhas do ciclo usam as mesmas cores em styles.css). */
const phaseColor = (s: PomodoroState) => (s.phase === "focus" ? "rgb(52 199 89)" : "rgb(64 156 255)");

const ResetIcon = () => (
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M3 12a9 9 0 1 0 3-6.7L3 8M3 3v5h5" />
  </svg>
);

/** Tempo restante: só este componente re-renderiza a cada segundo. */
function Clock({ state, settings, className }: { state: PomodoroState; settings: Settings; className?: string }) {
  const now = useNow(state.status === "running" ? 250 : 60_000);
  const text = formatClock(remainingMs(state, settings, now));
  return (
    <span className={className} style={text.length > 5 ? { fontSize: "0.72em" } : undefined}>
      {text}
    </span>
  );
}

function Ring({ state, settings }: { state: PomodoroState; settings: Settings }) {
  const now = useNow(state.status === "running" ? 250 : 60_000);
  const progress = 1 - remainingMs(state, settings, now) / phaseMs(state.phase, settings);
  const r = (RING - STROKE) / 2;
  const c = 2 * Math.PI * r;
  return (
    <svg className="pm-ring" width={RING} height={RING} data-running={state.status === "running"} style={{ color: phaseColor(state) }}>
      <circle cx={RING / 2} cy={RING / 2} r={r} fill="none" stroke="var(--track)" strokeWidth={STROKE} />
      <circle
        cx={RING / 2}
        cy={RING / 2}
        r={r}
        fill="none"
        stroke="currentColor"
        strokeWidth={STROKE}
        strokeLinecap="round"
        className="pm-ring-arc"
        strokeDasharray={c}
        strokeDashoffset={c * (1 - Math.min(1, Math.max(0, progress)))}
        transform={`rotate(-90 ${RING / 2} ${RING / 2})`}
      />
    </svg>
  );
}

/** Parado, o tempo vira um campo: "25", "90m", "1h30" ou "1:30" (1 min a 24 h). */
function ClockEditor({ minutes, onCommit }: { minutes: number; onCommit: (m: number) => void }) {
  const [text, setText] = useState<string | null>(null);
  const label = formatClock(minutes * 60_000);
  if (text === null) {
    return (
      <button className="pm-clock pm-clock-btn" onClick={() => setText(String(minutes))} data-tip="Escolher o tempo" style={label.length > 5 ? { fontSize: 17 } : undefined}>
        {label}
      </button>
    );
  }
  const commit = () => {
    const parsed = parseDuration(text);
    if (parsed !== null) onCommit(parsed);
    setText(null);
  };
  return (
    <input
      className="pm-clock-input"
      autoFocus
      value={text}
      size={6}
      inputMode="numeric"
      placeholder="min"
      aria-label="Duração (1 min a 24 h)"
      onFocus={(e) => e.currentTarget.select()}
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") commit();
        if (e.key === "Escape") setText(null);
      }}
    />
  );
}

/** Segurar o botão repete a ação: começa devagar e segue até soltar. */
function useHoldRepeat(tick: () => void) {
  const latest = useRef(tick);
  latest.current = tick;
  const timers = useRef<{ delay?: number; repeat?: number }>({});
  const stop = useCallback(() => {
    clearTimeout(timers.current.delay);
    clearInterval(timers.current.repeat);
  }, []);
  useEffect(() => stop, [stop]);
  const start = useCallback(() => {
    stop();
    latest.current();
    timers.current.delay = window.setTimeout(() => {
      timers.current.repeat = window.setInterval(() => latest.current(), 80);
    }, 350);
  }, [stop]);
  return { onPointerDown: start, onPointerUp: stop, onPointerLeave: stop, onPointerCancel: stop };
}

type ControlsProps = { state: PomodoroState; toggle: () => void; skip: () => void; reset: () => void };

type ExpandedProps = {
  state: PomodoroState;
  settings: Settings;
  update: (patch: Partial<Settings>) => void;
  topInset: number;
  controls: ControlsProps;
};

function Expanded({ state, settings, update, topInset, controls }: ExpandedProps) {
  const running = state.status === "running";
  const idle = state.status === "idle";
  const key = PHASE_KEY[state.phase];
  const minutes = settings[key];
  const setMinutes = (m: number) => update({ [key]: clampMinutes(m) });
  // direção do ajuste: o passo cresce com o tempo (1, 5, 15, 30 min) pra chegar em horas sem 20 cliques
  const adjust = (dir: 1 | -1) => setMinutes(minutes + dir * (dir < 0 ? stepFor(minutes - 1) : stepFor(minutes)));
  const holdMinus = useHoldRepeat(() => adjust(-1));
  const holdPlus = useHoldRepeat(() => adjust(1));
  const keyStep = (dir: 1 | -1) => (e: React.KeyboardEvent) => {
    if (e.key === "Enter" || e.key === " ") adjust(dir);
  };
  const next = nextPhase(state, settings.longEvery);
  return (
    <div className="pm-expanded" style={{ paddingTop: topInset }}>
      <div className="pm-ring-wrap" onWheel={idle ? (e) => adjust(e.deltaY < 0 ? 1 : -1) : undefined} data-tip={idle ? "Role pra ajustar o tempo" : undefined}>
        <Ring state={state} settings={settings} />
        <div className="pm-center">
          {idle ? <ClockEditor minutes={minutes} onCommit={setMinutes} /> : <Clock state={state} settings={settings} className="pm-clock" />}
          {idle ? (
            <span className="pm-steps">
              <button className="pm-step" {...holdMinus} onKeyDown={keyStep(-1)} disabled={minutes <= 1} data-tip="Menos (segure pra repetir)" aria-label="Menos">−</button>
              <button className="pm-step" {...holdPlus} onKeyDown={keyStep(1)} disabled={minutes >= 1440} data-tip="Mais (segure pra repetir)" aria-label="Mais">+</button>
            </span>
          ) : (
            <span className="pm-caption">{running ? "restante" : "pausado"}</span>
          )}
        </div>
      </div>
      <div className="pm-side">
        <div className="pm-head">
          <span className="pm-badge" style={{ color: phaseColor(state) }}>
            <i style={{ background: phaseColor(state) }} />
            {PHASE_LABEL[state.phase]}
          </span>
          <span className="pm-dots" data-dense={settings.longEvery > 5} data-tip="Fases do ciclo: foco, pausa, foco, pausa... e a pausa longa no fim">
            {Array.from({ length: settings.longEvery * 2 }, (_, i) => {
              const current = cycleIndex(state, settings.longEvery);
              return (
                <span
                  key={i}
                  data-kind={i % 2 === 0 ? "focus" : i === settings.longEvery * 2 - 1 ? "long" : "break"}
                  data-on={i <= current}
                  data-live={running && i === current}
                />
              );
            })}
          </span>
        </div>
        <div className="pm-info">
          {idle ? (
            <div className="pm-presets">
              {PRESETS[state.phase].map((p) => (
                <button key={p.minutes} className="pm-preset" data-on={p.minutes === minutes} onClick={() => setMinutes(p.minutes)}>
                  {p.label}
                </button>
              ))}
            </div>
          ) : (
            <>
              <span>{running ? `Termina às ${clockTime(state.endsAt)}` : "Em pausa"}</span>
              <span className="pm-next">Depois: {PHASE_LABEL[next].toLowerCase()}</span>
            </>
          )}
        </div>
        <div className="pm-controls">
          <motion.button className="pm-btn pm-btn-primary" onClick={controls.toggle} whileTap={{ scale: 0.92 }} data-tip={running ? "Pausar" : state.status === "paused" ? "Continuar" : "Iniciar"}>
            <AnimatePresence initial={false} mode="popLayout">
              <motion.span
                key={running ? "pause" : "play"}
                className="pm-btn-icon"
                initial={{ opacity: 0, scale: 0.5 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0, scale: 0.5 }}
                transition={{ duration: 0.14 }}
              >
                {running ? <PauseIcon size={22} /> : <PlayIcon size={22} />}
              </motion.span>
            </AnimatePresence>
          </motion.button>
          <motion.button className="pm-btn" onClick={controls.skip} whileTap={{ scale: 0.92 }} data-tip="Pular fase">
            <NextIcon size={16} />
          </motion.button>
          <motion.button className="pm-btn" onClick={controls.reset} disabled={idle && state.cycle === 0 && state.phase === "focus"} whileTap={{ scale: 0.92 }} data-tip="Zerar">
            <ResetIcon />
          </motion.button>
        </div>
      </div>
    </div>
  );
}

/** Pomodoro: aba no Expanded e, rodando ou pausado, o tempo no lado direito do Compact. */
export function usePomodoroFeature(settings: Settings, update: (patch: Partial<Settings>) => void): { activity: Activity | null; panel: Panel | null } {
  const { state, toggle, skip, reset } = usePomodoro();
  if (!settings.showPomodoro || !state) return { activity: null, panel: null };
  const active = state.status !== "idle";
  const activity: Activity | null = active
    ? {
        id: "pomodoro",
        compact: () => ({
          right: (
            <Clock
              state={state}
              settings={settings}
              className="pm-compact"
            />
          ),
        }),
      }
    : null;
  const panel: Panel = {
    id: "pomodoro",
    label: "Pomodoro",
    icon: <TimerIcon />,
    expanded: ({ topInset }) => <Expanded state={state} settings={settings} update={update} topInset={topInset} controls={{ state, toggle, skip, reset }} />,
    expandedSize: { width: WIDTH, height: HEIGHT },
  };
  return { activity, panel };
}
