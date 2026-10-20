import { AnimatePresence, motion } from "motion/react";
import { useEffect, useState } from "react";
import { useArtworkColor } from "../../hooks/useArtworkColor";
import { native, type Settings } from "../../lib/native";
import { MusicIcon } from "../../panels/icons";
import type { Activity, Panel } from "../types";
import { Equalizer, type EqualizerLook } from "./Equalizer";
import { NextIcon, PauseIcon, PlayIcon, PrevIcon, RepeatIcon, RepeatOneIcon, ShuffleIcon } from "./Icons";
import { Progress } from "./Progress";
import { VolumeIcon, VolumePanel } from "./Volume";
import { useNowPlaying, type NowPlayingControls } from "./useNowPlaying";

const REPEAT_LABEL = { off: "Repetir playlist", context: "Repetir só esta faixa", track: "Desativar repetição" } as const;

const EXPANDED_WIDTH = 400;
/** altura do equalizer na compacta, em fração da capa */
const COMPACT_EQ_SCALE = 0.85;
/** lado da capa no cabeçalho da aberta; o GIF do equalizer não passa disso */
const ART_EXPANDED = 56;
/** o GIF na aberta é tantas vezes o tamanho da compacta */
const GIF_EXPANDED_SCALE = 2;
/** cabeçalho + progresso + controles + respiro inferior */
const EXPANDED_BODY = 150;
/** espaço extra quando o painel de volume está aberto */
const VOLUME_PANEL = 50;

export function Artwork({ url, size, radius }: { url: string; size: number; radius: number }) {
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

export function CompactEqualizer({
  playing,
  height,
  color,
  look,
}: {
  playing: boolean;
  height: number;
  color: string;
  look: EqualizerLook;
}) {
  const art = Math.max(16, height - 12);
  // o GIF não pode passar da altura da Island: estourar a linha do grid empurra a capa pra baixo
  const compactLook = { ...look, equalizerGifSize: Math.min(look.equalizerGifSize, Math.max(16, height - 4)) };
  return <Equalizer playing={playing} color={color} height={Math.round(art * COMPACT_EQ_SCALE)} look={compactLook} />;
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
  look: EqualizerLook;
  volumeOpen: boolean;
  setVolumeOpen: (open: boolean) => void;
};

function Expanded({ np, topInset, color, look, volumeOpen, setVolumeOpen }: ExpandedProps) {
  const { state } = np.playback;
  const track = state.track!;
  const playing = state.status === "playing";
  // o GIF fica maior na aberta (o tamanho dos Ajustes vale pra compacta), sem passar da altura da capa
  const expandedLook = { ...look, equalizerGifSize: Math.min(look.equalizerGifSize * GIF_EXPANDED_SCALE, ART_EXPANDED) };
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
          <Artwork url={track.artworkUrl} size={ART_EXPANDED} radius={12} />
        </motion.button>
        <div className="np-meta">
          <div className="np-title" data-tip={track.name} data-tip-overflow>{track.name}</div>
          <div className="np-artist" data-tip={track.artist} data-tip-overflow>{track.artist}</div>
        </div>
        <Equalizer playing={playing} color={color} height={18} look={expandedLook} />
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
            right: <CompactEqualizer playing height={height} color={color} look={settings} />,
          }),
          theme: palette.theme,
        }
      : null;
  const panel: Panel = {
    id: "now-playing",
    label: "Música",
    icon: <MusicIcon />,
    expanded: ({ topInset }) => (
      <Expanded np={np} topInset={topInset} color={color} look={settings} volumeOpen={volumeOpen} setVolumeOpen={setVolumeOpen} />
    ),
    // a Island "escorre" pra baixo quando o painel de volume abre
    expandedSize: { width: EXPANDED_WIDTH, height: EXPANDED_BODY + (volumeOpen ? VOLUME_PANEL : 0) },
    theme: palette.theme,
  };
  return { activity, panel };
}
