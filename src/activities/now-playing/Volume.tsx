import { AnimatePresence, motion } from "motion/react";
import { useRef } from "react";
import { VolumeHighIcon, VolumeLowIcon, VolumeMutedIcon } from "./Icons";
import { Slider } from "./Slider";

/** Intervalo mínimo entre comandos durante o arrasto (cada um é um osascript). */
const THROTTLE_MS = 120;

export function VolumeIcon({ volume, size }: { volume: number; size?: number }) {
  const level = volume === 0 ? "muted" : volume < 50 ? "low" : "high";
  const Icon = { muted: VolumeMutedIcon, low: VolumeLowIcon, high: VolumeHighIcon }[level];
  return (
    <AnimatePresence initial={false} mode="popLayout">
      <motion.span
        key={level}
        className="np-icon-swap"
        initial={{ opacity: 0, scale: 0.6 }}
        animate={{ opacity: 1, scale: 1 }}
        exit={{ opacity: 0, scale: 0.6 }}
        transition={{ duration: 0.15 }}
      >
        <Icon size={size} />
      </motion.span>
    </AnimatePresence>
  );
}

type PanelProps = { volume: number; color: string; onChange: (v: number) => void; onToggleMute: () => void };

/** Painel que desce da Island ao clicar no botão de volume: mute, slider e porcentagem. */
export function VolumePanel({ volume, color, onChange, onToggleMute }: PanelProps) {
  const last = useRef(0);
  const throttled = (r: number) => {
    const now = Date.now();
    if (now - last.current < THROTTLE_MS) return;
    last.current = now;
    onChange(r * 100);
  };

  return (
    <motion.div
      className="np-volume-panel"
      initial={{ opacity: 0, y: -8, filter: "blur(6px)" }}
      animate={{ opacity: 1, y: 0, filter: "blur(0px)", transition: { duration: 0.22, delay: 0.05 } }}
      exit={{ opacity: 0, y: -8, filter: "blur(6px)", transition: { duration: 0.12 } }}
    >
      <motion.button
        className="np-mute"
        data-muted={volume === 0}
        onClick={onToggleMute}
        whileTap={{ scale: 0.85 }}
        aria-label={volume === 0 ? "Desmutar" : "Mutar"}
      >
        <VolumeIcon volume={volume} size={18} />
      </motion.button>
      <Slider value={volume / 100} color={color} onChange={throttled} onCommit={(r) => onChange(r * 100)} />
      <span className="np-volume-value">{volume}%</span>
    </motion.div>
  );
}
