import { useEffect, useState } from "react";

const FALLBACK = "rgb(255 255 255)";

/** Cor de destaque da capa: média ponderada por saturação, clareada pra ler bem no preto. */
export function useArtworkColor(url: string | undefined): string {
  const [color, setColor] = useState(FALLBACK);
  useEffect(() => {
    if (!url) return setColor(FALLBACK);
    let cancelled = false;
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.onload = () => {
      if (cancelled) return;
      try {
        const size = 16;
        const ctx = document.createElement("canvas").getContext("2d", { willReadFrequently: true });
        if (!ctx) return;
        ctx.canvas.width = ctx.canvas.height = size;
        ctx.drawImage(img, 0, 0, size, size);
        const px = ctx.getImageData(0, 0, size, size).data;
        let r = 0, g = 0, b = 0, total = 0;
        for (let i = 0; i < px.length; i += 4) {
          const max = Math.max(px[i], px[i + 1], px[i + 2]);
          const min = Math.min(px[i], px[i + 1], px[i + 2]);
          const w = (max - min) / 255 + 0.05;
          r += px[i] * w; g += px[i + 1] * w; b += px[i + 2] * w; total += w;
        }
        const avg = [r, g, b].map((c) => c / total);
        const lift = 200 / Math.max(...avg, 1);
        const [cr, cg, cb] = avg.map((c) => Math.round(Math.min(255, c * Math.max(lift, 1))));
        setColor(`rgb(${cr} ${cg} ${cb})`);
      } catch {
        setColor(FALLBACK);
      }
    };
    img.onerror = () => !cancelled && setColor(FALLBACK);
    img.src = url;
    return () => {
      cancelled = true;
    };
  }, [url]);
  return color;
}
