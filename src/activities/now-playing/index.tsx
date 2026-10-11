import { AnimatePresence, motion } from "motion/react";
import { useEffect, useState } from "react";
import { useArtworkColor } from "../../hooks/useArtworkColor";
import { native, type Settings } from "../../lib/native";
import { MusicIcon } from "../../panels/icons";
import type { Activity, Panel } from "../types";
import { Equalizer } from "./Equalizer";
import { NextIcon, PauseIcon, PlayIcon, PrevIcon, RepeatIcon, RepeatOneIcon, ShuffleIcon } from "./Icons";
import { Progress } from "./Progress";
import { VolumeIcon, VolumePanel } from "./Volume";
import { useNowPlaying, type NowPlayingControls } from "./useNowPlaying";

const REPEAT_LABEL = { off: "Repetir playlist", context: "Repetir só esta faixa", track: "Desativar repetição" } as const;

const EXPANDED_WIDTH = 400;
/** cabeçalho + progresso + controles + respiro inferior */
const EXPANDED_BODY = 150;
/** espaço extra quando o painel de volume está aberto */
const VOLUME_PANEL = 50;

function Artwork({ url, size, radius }: { url: string; size: number; radius: number }) {
  return (
    <AnimatePresence initial={false} mode="popLayout">
      <motion.img
        key={url}
        src={url}
        className="np-art"
        style={{ width: size, height: size, borderRadius: radius }}
        initial={{ opacity: 0, scale: 0.85, filter: "blur(6px)" }}
        animate={{ opacity: 1, scale: 1, filter: "blur(0px)" }}
        exit={{ opacity: 0, scale: 0.85, filter: "blur(6px)" }}
        transition={{ duration: 0.3 }}
        draggable={false}
      />
    </AnimatePresence>
  );
}

function CompactArt({ np, height }: { np: NowPlayingControls; height: number }) {
  const art = Math.max(16, height - 12);
  return <Artwork url={np.playback.state.track!.artworkUrl} size={art} radius={6} />;
}

function CompactEqualizer({ np, height, color }: { np: NowPlayingControls; height: number; color: string }) {
  const art = Math.max(16, height - 12);
  return <Equalizer playing={np.playback.state.status === "playing"} color={color} height={Math.round(art * 0.6)} />;
}

type ButtonProps = {
  onClick: () => void;
  children: React.ReactNode;
  primary?: boolean;
  active?: boolean;
  label?: string;
  /** botão de modo (shuffle/repeat): ligado fica na cor de destaque com um ponto embaixo */
  on?: boolean;
  color?: string;
  small?: boolean;
};

function Button({ onClick, children, primary, active, label, on, color, small }: ButtonProps) {
  return (
    <motion.button
      className="np-btn"
      data-primary={primary}
      data-small={small}
      data-on={on}
      style={on ? { color } : undefined}
      data-tip={label}
      data-active={active}
      aria-label={label}
      onClick={onClick}
      whileHover={{ scale: 1.08 }}
      whileTap={{ scale: 0.82 }}
      transition={{ type: "spring", stiffness: 600, damping: 22 }}
    >
      {children}
    </motion.button>
  );
}

type ExpandedProps = {
  np: NowPlayingControls;
  topInset: number;
  color: string;
  volumeOpen: boolean;
  setVolumeOpen: (open: boolean) => void;
};

function Expanded({ np, topInset, color, volumeOpen, setVolumeOpen }: ExpandedProps) {
  const { state } = np.playback;
  const track = state.track!;
  const playing = state.status === "playing";
  // a Island fechou: na próxima abertura o painel de volume começa recolhido
  useEffect(() => () => setVolumeOpen(false), [setVolumeOpen]);
  return (
    <div className="np-expanded" style={{ paddingTop: topInset }}>
      <div className="np-head">
        <motion.button
          className="np-art-btn"
          onClick={() => native.spotifyOpen(track.id)}
          data-tip="Abrir no Spotify"
          aria-label="Abrir no Spotify"
          whileHover={{ scale: 1.05 }}
          whileTap={{ scale: 0.92 }}
          transition={{ type: "spring", stiffness: 600, damping: 22 }}
        >
          <Artwork url={track.artworkUrl} size={56} radius={12} />
        </motion.button>
        <div className="np-meta">
          <div className="np-title" data-tip={track.name} data-tip-overflow>{track.name}</div>
          <div className="np-artist" data-tip={track.artist} data-tip-overflow>{track.artist}</div>
        </div>
        <Equalizer playing={playing} color={color} height={18} />
      </div>
      <Progress
        playback={np.playback}
        color={color}
        onSeek={np.seek}
        trailing={
          <Button onClick={() => setVolumeOpen(!volumeOpen)} active={volumeOpen} small label={volumeOpen ? "Fechar volume" : "Volume"}>
            <VolumeIcon volume={state.volume} size={16} />
          </Button>
        }
      />
      <div className="np-controls">
        <Button onClick={np.toggleShuffle} on={state.shuffle} color={color} label={state.shuffle ? "Desativar aleatório" : "Ativar aleatório"}>
          <ShuffleIcon />
        </Button>
        <Button onClick={np.previous} label="Faixa anterior"><PrevIcon /></Button>
        <Button onClick={np.toggle} primary label={playing ? "Pausar" : "Reproduzir"}>
          <AnimatePresence initial={false} mode="popLayout">
            <motion.span
              key={playing ? "pause" : "play"}
              className="np-icon-swap"
              initial={{ opacity: 0, scale: 0.4 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.4 }}
              transition={{ duration: 0.18 }}
            >
              {playing ? <PauseIcon size={26} /> : <PlayIcon size={26} />}
            </motion.span>
          </AnimatePresence>
        </Button>
        <Button onClick={np.next} label="Próxima faixa"><NextIcon /></Button>
        <Button onClick={np.cycleRepeat} on={state.repeat !== "off"} color={color} label={REPEAT_LABEL[state.repeat]}>
          {state.repeat === "track" ? <RepeatOneIcon /> : <RepeatIcon />}
        </Button>
      </div>
      <AnimatePresence initial={false}>
        {volumeOpen && (
          <VolumePanel
            key="volume"
            volume={state.volume}
            color={color}
            onChange={np.setVolume}
            onToggleMute={np.toggleMute}
          />
        )}
      </AnimatePresence>
    </div>
  );
}

/** Now Playing do Spotify: Activity no Compact enquanto toca, aba no Expanded também quando pausado. */
export function useNowPlayingFeature(settings: Settings): { activity: Activity | null; panel: Panel | null } {
  const np = useNowPlaying();
  const [volumeOpen, setVolumeOpen] = useState(false);
  const { state } = np.playback;
  const palette = useArtworkColor(state.track?.artworkUrl);
  // no Themed o fundo já é a cor da capa: o destaque segue o texto pra manter contraste
  const color =
    settings.style === "themed" && palette.theme
      ? "var(--text)"
      : settings.accent === "white"
        ? "rgb(255 255 255)"
        : palette.accent;
  if (!state.track || (state.status !== "playing" && state.status !== "paused")) return { activity: null, panel: null };
  const activity: Activity | null =
    state.status === "playing"
      ? {
          id: "now-playing",
          compact: ({ height }) => ({
            left: <CompactArt np={np} height={height} />,
            right: <CompactEqualizer np={np} height={height} color={color} />,
          }),
          theme: palette.theme,
        }
      : null;
  const panel: Panel = {
    id: "now-playing",
    label: "Música",
    icon: <MusicIcon />,
    expanded: ({ topInset }) => (
      <Expanded np={np} topInset={topInset} color={color} volumeOpen={volumeOpen} setVolumeOpen={setVolumeOpen} />
    ),
    // a Island "escorre" pra baixo quando o painel de volume abre
    expandedSize: { width: EXPANDED_WIDTH, height: EXPANDED_BODY + (volumeOpen ? VOLUME_PANEL : 0) },
    theme: palette.theme,
  };
  return { activity, panel };
}
