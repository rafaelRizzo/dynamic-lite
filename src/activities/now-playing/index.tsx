import { AnimatePresence, motion } from "motion/react";
import { useEffect, useState } from "react";
import { useArtworkColor } from "../../hooks/useArtworkColor";
import { native, type Settings } from "../../lib/native";
import type { Activity } from "../types";
import { Equalizer } from "./Equalizer";
import { NextIcon, PauseIcon, PlayIcon, PrevIcon, RepeatIcon, RepeatOneIcon, ShuffleIcon } from "./Icons";
import { Progress } from "./Progress";
import { VolumeIcon, VolumePanel } from "./Volume";
import { useNowPlaying, type NowPlayingControls } from "./useNowPlaying";

const REPEAT_LABEL = { off: "Ativar repetir", context: "Repetir só esta faixa", track: "Desativar repetir" } as const;

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

function Compact({ np, side, height, color }: { np: NowPlayingControls; side: number; height: number; color: string }) {
  const { state } = np.playback;
  const art = Math.max(16, height - 12);
  return (
    <div className="np-compact" style={{ paddingInline: (side - art) / 2 }}>
      <Artwork url={state.track!.artworkUrl} size={art} radius={6} />
      <Equalizer playing={state.status === "playing"} color={color} height={Math.round(art * 0.6)} />
    </div>
  );
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
      title={label}
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
          title="Abrir no Spotify"
          whileHover={{ scale: 1.05 }}
          whileTap={{ scale: 0.92 }}
          transition={{ type: "spring", stiffness: 600, damping: 22 }}
        >
          <Artwork url={track.artworkUrl} size={56} radius={12} />
        </motion.button>
        <div className="np-meta">
          <div className="np-title" title={track.name}>{track.name}</div>
          <div className="np-artist" title={track.artist}>{track.artist}</div>
        </div>
        <Equalizer playing={playing} color={color} height={18} />
      </div>
      <Progress
        playback={np.playback}
        color={color}
        onSeek={np.seek}
        trailing={
          <Button onClick={() => setVolumeOpen(!volumeOpen)} active={volumeOpen} small label="Volume">
            <VolumeIcon volume={state.volume} size={16} />
          </Button>
        }
      />
      <div className="np-controls">
        <Button onClick={np.toggleShuffle} on={state.shuffle} color={color} label={state.shuffle ? "Desativar aleatório" : "Ativar aleatório"}>
          <ShuffleIcon />
        </Button>
        <Button onClick={np.previous} label="Anterior"><PrevIcon /></Button>
        <Button onClick={np.toggle} primary>
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
        <Button onClick={np.next} label="Próxima"><NextIcon /></Button>
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

/** Now Playing do Spotify como Activity: live tocando, disponível no hover quando pausado. */
export function useNowPlayingActivity(settings: Settings): Activity | null {
  const np = useNowPlaying();
  const [volumeOpen, setVolumeOpen] = useState(false);
  const { state } = np.playback;
  const artworkColor = useArtworkColor(state.track?.artworkUrl);
  const color = settings.accent === "white" ? "rgb(255 255 255)" : artworkColor;
  if (!state.track || (state.status !== "playing" && state.status !== "paused")) return null;
  return {
    id: "now-playing",
    live: state.status === "playing",
    compact: ({ side, height }) => <Compact np={np} side={side} height={height} color={color} />,
    expanded: ({ topInset }) => (
      <Expanded np={np} topInset={topInset} color={color} volumeOpen={volumeOpen} setVolumeOpen={setVolumeOpen} />
    ),
    // a Island "escorre" pra baixo quando o painel de volume abre
    expandedSize: { width: EXPANDED_WIDTH, height: EXPANDED_BODY + (volumeOpen ? VOLUME_PANEL : 0) },
  };
}
