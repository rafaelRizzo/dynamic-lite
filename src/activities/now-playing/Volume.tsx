import { useRef } from "react";
import { VolumeHighIcon, VolumeLowIcon } from "./Icons";
import { Slider } from "./Slider";

/** Intervalo mínimo entre comandos durante o arrasto (cada um é um osascript). */
const THROTTLE_MS = 120;

export function Volume({ volume, onChange }: { volume: number; onChange: (v: number) => void }) {
  const last = useRef(0);
  const throttled = (r: number) => {
    const now = Date.now();
    if (now - last.current < THROTTLE_MS) return;
    last.current = now;
    onChange(r * 100);
  };

  return (
    <div className="np-row np-volume">
      <button className="np-vol-btn" onClick={() => onChange(volume - 10)} aria-label="Diminuir volume">
        <VolumeLowIcon />
      </button>
      <Slider value={volume / 100} color="rgb(255 255 255 / 0.9)" onChange={throttled} onCommit={(r) => onChange(r * 100)} />
      <button className="np-vol-btn" onClick={() => onChange(volume + 10)} aria-label="Aumentar volume">
        <VolumeHighIcon />
      </button>
    </div>
  );
}
