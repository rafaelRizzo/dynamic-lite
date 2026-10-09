import { useEffect, useRef } from "react";
import { native, subscribe } from "../../lib/native";

/** Velocidade de cada barra na animação de fallback (sem níveis ao vivo). */
const BARS = [0.9, 0.62, 1.05, 0.75];
/** Sem níveis por esse tempo, volta pra animação CSS. */
const LIVE_TIMEOUT = 400;
const MIN_SCALE = 0.2;

export function Equalizer({ playing, color, height = 14 }: { playing: boolean; color: string; height?: number }) {
  const ref = useRef<HTMLDivElement>(null);

  // níveis ao vivo direto no DOM: ~30 eventos/s sem re-render
  useEffect(() => {
    const el = ref.current;
    if (!el || !playing) return;
    const bars = Array.from(el.children) as HTMLElement[];
    let timer: number | undefined;
    const reset = () => {
      delete el.dataset.live;
      bars.forEach((b) => (b.style.transform = ""));
    };
    const unsubscribe = subscribe(
      native.onLevels((levels) => {
        el.dataset.live = "";
        bars.forEach((b, i) => (b.style.transform = `scaleY(${MIN_SCALE + (1 - MIN_SCALE) * (levels[i] ?? 0)})`));
        clearTimeout(timer);
        timer = window.setTimeout(reset, LIVE_TIMEOUT);
      }),
    );
    return () => {
      unsubscribe();
      clearTimeout(timer);
      reset();
    };
  }, [playing]);

  return (
    <div ref={ref} className="eq" data-playing={playing} style={{ height, color }}>
      {BARS.map((speed, i) => (
        <span key={i} style={{ animationDuration: `${speed}s`, animationDelay: `${-i * 0.23}s` }} />
      ))}
    </div>
  );
}
