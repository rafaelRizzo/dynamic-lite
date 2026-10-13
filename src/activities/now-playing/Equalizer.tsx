import { useEffect, useRef } from "react";
import { native, subscribe } from "../../lib/native";

const BARS = 4;
const BAR_W = 3;
const GAP = 2;
const WIDTH = BARS * BAR_W + (BARS - 1) * GAP;
const MIN_SCALE = 0.2;
/** Velocidade (rad/s) de cada barra na animação de fallback (sem níveis ao vivo). */
const IDLE_SPEED = [6.9, 10.1, 5.9, 8.4];
/** Sem níveis por esse tempo, volta pra animação de fallback. */
const LIVE_TIMEOUT = 400;
/** Quadros por segundo do desenho. */
const FPS = 20;
/** Suavização por quadro rumo ao alvo (0-1). */
const SMOOTH = 0.45;

export function Equalizer({ playing, color, height = 14 }: { playing: boolean; color: string; height?: number }) {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = ref.current;
    const ctx = canvas?.getContext("2d");
    if (!canvas || !ctx) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = WIDTH * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);
    ctx.fillStyle = getComputedStyle(canvas).color;

    const level = new Array(BARS).fill(MIN_SCALE);
    const shown = new Array(BARS).fill(MIN_SCALE);
    let liveUntil = 0;

    const draw = () => {
      ctx.clearRect(0, 0, WIDTH, height);
      for (let i = 0; i < BARS; i++) {
        const h = Math.max(2, shown[i] * height);
        ctx.beginPath();
        ctx.roundRect(i * (BAR_W + GAP), (height - h) / 2, BAR_W, h, 1.5);
        ctx.fill();
      }
    };

    if (!playing) {
      shown.fill(MIN_SCALE);
      draw();
      return;
    }

    const unsubscribe = subscribe(
      native.onLevels((levels) => {
        liveUntil = performance.now() + LIVE_TIMEOUT;
        for (let i = 0; i < BARS; i++) level[i] = MIN_SCALE + (1 - MIN_SCALE) * (levels[i] ?? 0);
      }),
    );
    const tick = () => {
      const now = performance.now();
      const live = now < liveUntil;
      for (let i = 0; i < BARS; i++) {
        const target = live ? level[i] : MIN_SCALE + (1 - MIN_SCALE) * (0.5 + 0.5 * Math.sin((now / 1000) * IDLE_SPEED[i] + i * 1.3));
        shown[i] += (target - shown[i]) * SMOOTH;
      }
      draw();
    };
    const timer = window.setInterval(tick, 1000 / FPS);
    return () => {
      unsubscribe();
      clearInterval(timer);
    };
  }, [playing, height, color]);

  return <canvas ref={ref} className="eq" style={{ width: WIDTH, height, color }} />;
}
