import { AnimatePresence, motion } from "motion/react";
import { useArtworkColor } from "../../hooks/useArtworkColor";
import type { Settings } from "../../lib/native";
import type { Activity } from "../types";
import { Equalizer } from "./Equalizer";
import { NextIcon, PauseIcon, PlayIcon, PrevIcon } from "./Icons";
import { Progress } from "./Progress";
import { Volume } from "./Volume";
import { useNowPlaying, type NowPlayingControls } from "./useNowPlaying";

const EXPANDED_WIDTH = 400;
/** cabeçalho + progresso + controles + volume + respiro inferior */
const EXPANDED_BODY = 184;

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

function Button({ onClick, children, primary }: { onClick: () => void; children: React.ReactNode; primary?: boolean }) {
  return (
    <motion.button
      className="np-btn"
      data-primary={primary}
      onClick={onClick}
      whileHover={{ scale: 1.08 }}
      whileTap={{ scale: 0.82 }}
      transition={{ type: "spring", stiffness: 600, damping: 22 }}
    >
      {children}
    </motion.button>
  );
}

function Expanded({ np, topInset, color }: { np: NowPlayingControls; topInset: number; color: string }) {
  const { state } = np.playback;
  const track = state.track!;
  const playing = state.status === "playing";
  return (
    <div className="np-expanded" style={{ paddingTop: topInset }}>
      <div className="np-head">
        <Artwork url={track.artworkUrl} size={56} radius={12} />
        <div className="np-meta">
          <div className="np-title" title={track.name}>{track.name}</div>
          <div className="np-artist" title={track.artist}>{track.artist}</div>
        </div>
        <Equalizer playing={playing} color={color} height={18} />
      </div>
      <Progress playback={np.playback} color={color} onSeek={np.seek} />
      <div className="np-controls">
        <Button onClick={np.previous}><PrevIcon /></Button>
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
        <Button onClick={np.next}><NextIcon /></Button>
      </div>
      <Volume volume={state.volume} onChange={np.setVolume} />
    </div>
  );
}

/** Now Playing do Spotify como Activity: live tocando, disponível no hover quando pausado. */
export function useNowPlayingActivity(settings: Settings): Activity | null {
  const np = useNowPlaying();
  const { state } = np.playback;
  const artworkColor = useArtworkColor(state.track?.artworkUrl);
  const color = settings.accent === "white" ? "rgb(255 255 255)" : artworkColor;
  if (!state.track || (state.status !== "playing" && state.status !== "paused")) return null;
  return {
    id: "now-playing",
    live: state.status === "playing",
    compact: ({ side, height }) => <Compact np={np} side={side} height={height} color={color} />,
    expanded: ({ topInset }) => <Expanded np={np} topInset={topInset} color={color} />,
    expandedSize: { width: EXPANDED_WIDTH, height: EXPANDED_BODY },
  };
}
