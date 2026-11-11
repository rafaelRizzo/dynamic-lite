import { motion } from "motion/react";
import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { native, type Track } from "../../lib/native";
import { lineAt, parseLrc, type LyricLine } from "../../lib/lyrics";
import { positionAt, type Playback } from "./useNowPlaying";

/** A posição do Spotify chega com atraso (leitura a cada ~1 s): adianta a troca de linha um pouco, além do ajuste dos Ajustes. */
const LEAD_S = 0.2;

type Loaded =
  | { status: "loading" }
  | { status: "error" }
  | { status: "none" }
  | { status: "synced"; lines: LyricLine[] }
  | { status: "plain"; text: string };

/** Letra da faixa, buscada só enquanto `enabled` (painel aberto). Respostas de faixas antigas são descartadas. */
function useLyrics(track: Track, enabled: boolean): Loaded {
  const [loaded, setLoaded] = useState<{ id: string; value: Loaded } | null>(null);
  useEffect(() => {
    if (!enabled) return;
    let stale = false;
    native
      .lyricsGet(track.name, track.artist, track.album, track.duration)
      .then((result): Loaded => {
        const lines = result?.syncedLyrics ? parseLrc(result.syncedLyrics) : [];
        if (lines.some((l) => l.text)) return { status: "synced", lines };
        const plain = result?.plainLyrics?.trim();
        return plain ? { status: "plain", text: plain } : { status: "none" };
      })
      .catch((): Loaded => ({ status: "error" }))
      .then((value) => {
        if (!stale) setLoaded({ id: track.id, value });
      });
    return () => {
      stale = true;
    };
  }, [enabled, track.id, track.name, track.artist, track.album, track.duration]);
  return loaded?.id === track.id ? loaded.value : { status: "loading" };
}

/** Linha atual da letra: troca por timer no instante da próxima linha, sem relógio girando à toa. */
function useLineIndex(playback: Playback, lines: LyricLine[], offsetS: number): number {
  const [index, setIndex] = useState(-1);
  useEffect(() => {
    let timer: number | undefined;
    const sync = () => {
      const position = positionAt(playback, Date.now()) + LEAD_S + offsetS;
      const current = lineAt(lines, position);
      setIndex(current);
      const next = lines[current + 1];
      // pausado: espera a próxima leitura (retomar, seek) mudar `playback`
      if (next && playback.state.status === "playing") {
        timer = window.setTimeout(sync, Math.max(30, (next.time - position) * 1000));
      }
    };
    sync();
    return () => clearTimeout(timer);
  }, [playback, lines, offsetS]);
  return index;
}

function Message({ children }: { children: string }) {
  return <div className="np-lyr-msg">{children}</div>;
}

type ListProps = {
  playback: Playback;
  lines: LyricLine[];
  color: string;
  offsetS: number;
  onSeek: (seconds: number) => void;
};

/** Todos os versos numa coluna: o atual fica no centro e em destaque, os outros apagados. Clicar pula pra ele. */
function LyricList({ playback, lines, color, offsetS, onSeek }: ListProps) {
  const index = useLineIndex(playback, lines, offsetS);
  const box = useRef<HTMLDivElement>(null);
  const rail = useRef<HTMLDivElement>(null);
  const [shift, setShift] = useState(0);
  // a altura da ilha muda (volume abre e fecha): recalcula o centro quando a caixa muda de tamanho
  const [height, setHeight] = useState(0);
  useLayoutEffect(() => {
    const el = box.current;
    if (!el) return;
    const observer = new ResizeObserver(() => setHeight(el.clientHeight));
    observer.observe(el);
    return () => observer.disconnect();
  }, []);
  useLayoutEffect(() => {
    const current = rail.current?.children[Math.max(index, 0)] as HTMLElement | undefined;
    if (!current || height === 0) return;
    setShift(height / 2 - (current.offsetTop + current.offsetHeight / 2));
  }, [index, lines, height]);

  return (
    <div ref={box} className="np-lyr" style={{ background: `linear-gradient(180deg, color-mix(in srgb, ${color} 30%, transparent), color-mix(in srgb, ${color} 14%, transparent))` }}>
      <div className="np-lyr-view">
      <div ref={rail} className="np-lyr-rail" style={{ transform: `translateY(${shift}px)` }}>
        {lines.map((line, i) => (
          <div
            key={i}
            className="np-lyr-line"
            data-on={i === index || undefined}
            style={{ "--d": Math.min(Math.abs(i - index), 3), ...(i === index ? { color, textShadow: `0 0 18px color-mix(in srgb, ${color} 55%, transparent)` } : null) } as React.CSSProperties}
            onClick={() => onSeek(line.time)}
          >
            {line.text || "♪"}
          </div>
        ))}
      </div>
      </div>
    </div>
  );
}

type Props = {
  playback: Playback;
  track: Track;
  color: string;
  offsetMs: number;
  onSeek: (seconds: number) => void;
  width: number;
  /** faixa do Notch: a coluna começa abaixo dela, alinhada ao cabeçalho do player */
  topInset: number;
};

/** Coluna de letra à direita do player: a ilha cresce na largura enquanto ela está aberta. */
export function LyricsSide({ playback, track, color, offsetMs, onSeek, width, topInset }: Props) {
  const lyrics = useLyrics(track, true);
  let body;
  switch (lyrics.status) {
    case "loading":
      body = <Message>Buscando letra…</Message>;
      break;
    case "error":
      body = <Message>Não foi possível buscar a letra</Message>;
      break;
    case "none":
      body = <Message>Letra não encontrada</Message>;
      break;
    case "plain":
      body = <div className="np-lyr-plain">{lyrics.text}</div>;
      break;
    case "synced":
      body = <LyricList playback={playback} lines={lyrics.lines} color={color} offsetS={offsetMs / 1000} onSeek={onSeek} />;
      break;
  }
  return (
    <motion.div
      className="np-lyr-side"
      style={{ width, paddingTop: topInset }}
      initial={{ opacity: 0, x: 18 }}
      animate={{ opacity: 1, x: 0, transition: { duration: 0.24, delay: 0.08 } }}
      exit={{ opacity: 0, x: 18, transition: { duration: 0.12 } }}
    >
      {body}
    </motion.div>
  );
}
