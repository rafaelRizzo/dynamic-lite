import { useState, type ReactNode } from "react";
import { useNow } from "../../hooks/useNow";
import { Slider } from "./Slider";
import { positionAt, type Playback } from "./useNowPlaying";

const fmt = (s: number) => {
  const t = Math.max(0, Math.floor(s));
  return `${Math.floor(t / 60)}:${String(t % 60).padStart(2, "0")}`;
};

type ProgressProps = {
  playback: Playback;
  color: string;
  onSeek: (s: number) => void;
  /** algo depois do tempo restante (ex.: botão de volume) */
  trailing?: ReactNode;
};

export function Progress({ playback, color, onSeek, trailing }: ProgressProps) {
  const now = useNow(250);
  const [preview, setPreview] = useState<number | null>(null);
  const duration = playback.state.track?.duration ?? 0;
  const position = preview ?? positionAt(playback, now);

  return (
    <div className="np-row">
      <span className="np-time">{fmt(position)}</span>
      <Slider
        value={duration > 0 ? position / duration : 0}
        color={color}
        onChange={(r) => setPreview(r * duration)}
        onCommit={(r) => {
          onSeek(r * duration);
          setPreview(null);
        }}
        onCancel={() => setPreview(null)}
      />
      <span className="np-time">-{fmt(duration - position)}</span>
      {trailing}
    </div>
  );
}
