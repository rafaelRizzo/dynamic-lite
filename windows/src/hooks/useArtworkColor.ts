import { useEffect, useState } from "react";

/** Fundo do Expanded no Style Themed e a cor do texto que dá contraste nele. */
export type Theme = { background: string; ink: "light" | "dark" };

export type ArtworkPalette = {
  /** destaque pra ler bem no preto (progresso, equalizer) */
  accent: string;
  theme: Theme | null;
};

const FALLBACK: ArtworkPalette = { accent: "rgb(255 255 255)", theme: null };

type RGB = [number, number, number];

const rgb = ([r, g, b]: RGB) => `rgb(${Math.round(r)} ${Math.round(g)} ${Math.round(b)})`;

function toHsl([r, g, b]: RGB): RGB {
  [r, g, b] = [r / 255, g / 255, b / 255];
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l];
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  const h = max === r ? (g - b) / d + (g < b ? 6 : 0) : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  return [h / 6, s, l];
}

function fromHsl([h, s, l]: RGB): RGB {
  const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
  const p = 2 * l - q;
  const hue = (t: number) => {
    t = (t + 1) % 1;
    if (t < 1 / 6) return p + (q - p) * 6 * t;
    if (t < 1 / 2) return q;
    if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
    return p;
  };
  return [hue(h + 1 / 3), hue(h), hue(h - 1 / 3)].map((c) => c * 255) as RGB;
}

/** Luminância relativa (WCAG). */
function luminance(c: RGB): number {
  const [r, g, b] = c.map((v) => {
    v /= 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** Cor da capa como fundo: sem extremos de claro/escuro e saturação contida; texto claro ou escuro pelo contraste. */
function themeFrom(avg: RGB): Theme {
  const [h, s, l] = toHsl(avg);
  const bg = fromHsl([h, Math.min(s, 0.7), Math.min(Math.max(l, 0.22), 0.62)]);
  const lum = luminance(bg);
  const onWhite = 1.05 / (lum + 0.05);
  const onBlack = (lum + 0.05) / 0.05;
  return { background: rgb(bg), ink: onWhite >= onBlack ? "light" : "dark" };
}

/** Paleta da capa: média ponderada por saturação, vira destaque (clareado) e fundo tematizado. */
export function useArtworkColor(url: string | undefined): ArtworkPalette {
  const [palette, setPalette] = useState(FALLBACK);
  useEffect(() => {
    if (!url) return setPalette(FALLBACK);
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
        const avg = [r, g, b].map((c) => c / total) as RGB;
        const lift = Math.max(200 / Math.max(...avg, 1), 1);
        const accent = avg.map((c) => Math.min(255, c * lift)) as RGB;
        setPalette({ accent: rgb(accent), theme: themeFrom(avg) });
      } catch {
        setPalette(FALLBACK);
      }
    };
    img.onerror = () => !cancelled && setPalette(FALLBACK);
    img.src = url;
    return () => {
      cancelled = true;
    };
  }, [url]);
  return palette;
}
