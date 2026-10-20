import { useEffect, useState } from "react";
import { useArtworkColor } from "../../hooks/useArtworkColor";
import { native, subscribe, type Settings, type SystemMedia } from "../../lib/native";
import { MusicIcon } from "../../panels/icons";
import { Artwork, CompactEqualizer } from "../now-playing";
import { Equalizer } from "../now-playing/Equalizer";
import type { Activity, Panel } from "../types";

const EXPANDED = { width: 400, height: 96 };
const ART_EXPANDED = 72;

/** Som audível no sistema e o que se sabe dele (o Rust só liga isso com a opção ativa e o Spotify parado). */
function useSystemAudio(enabled: boolean): { active: boolean; media: SystemMedia | null } {
  const [active, setActive] = useState(false);
  const [media, setMedia] = useState<SystemMedia | null>(null);
  useEffect(() => {
    if (!enabled) {
      setActive(false);
      setMedia(null);
      return;
    }
    native.equalizerSystemActive().then(setActive);
    native.systemMedia().then(setMedia);
    const offActive = subscribe(native.onSystemAudio(setActive));
    const offMedia = subscribe(native.onSystemMedia(setMedia));
    return () => {
      offActive();
      offMedia();
    };
  }, [enabled]);
  return { active: enabled && active, media };
}

function NoteBadge({ size }: { size: number }) {
  return (
    <span className="sa-badge" style={{ width: size, height: size }}>
      <MusicIcon />
    </span>
  );
}

function Expanded({ media, topInset, look, color }: { media: SystemMedia | null; topInset: number; look: Settings; color: string }) {
  const title = media?.title ?? media?.site ?? media?.app ?? "Áudio do sistema";
  const subtitle = media?.title ? [media.site, media.app].filter(Boolean).join(" · ") : media?.site ? media.app : "Tocando agora";
  const expandedLook = { ...look, equalizerGifSize: Math.min(look.equalizerGifSize * 2, ART_EXPANDED) };
  return (
    <div className="np-expanded sa-expanded" style={{ paddingTop: topInset }}>
      <div className="np-head">
        {media?.artworkUrl ? <Artwork url={media.artworkUrl} size={ART_EXPANDED} radius={14} /> : <NoteBadge size={ART_EXPANDED} />}
        <div className="np-meta">
          <div className="np-title" data-tip={title} data-tip-overflow>{title}</div>
          <div className="np-artist" data-tip={subtitle} data-tip-overflow>{subtitle}</div>
        </div>
        <Equalizer playing color={color} height={24} look={expandedLook} />
      </div>
    </div>
  );
}

/** Áudio de qualquer app (YouTube, Apple Music...): Activity no Compact e aba enquanto houver som; título e miniatura só de abas reconhecidas. */
export function useSystemAudioFeature(settings: Settings): { activity: Activity | null; panel: Panel | null } {
  const { active, media } = useSystemAudio(settings.systemAudio);
  const palette = useArtworkColor(media?.artworkUrl ?? undefined);
  if (!active) return { activity: null, panel: null };
  const theme = media?.artworkUrl ? palette.theme : null;
  // mesma regra do Now Playing; sem miniatura (só nota), a paleta cai no branco
  const color =
    settings.style === "themed" && theme ? "var(--text)" : settings.accent === "white" ? "rgb(255 255 255)" : palette.accent;
  const activity: Activity = {
    id: "system-audio",
    compact: ({ height }) => {
      const art = Math.max(16, height - 12);
      return {
        left: media?.artworkUrl ? <Artwork url={media.artworkUrl} size={art} radius={6} /> : <NoteBadge size={art} />,
        right: <CompactEqualizer playing height={height} color={color} look={settings} />,
      };
    },
    theme,
  };
  const panel: Panel = {
    id: "system-audio",
    label: "Áudio",
    icon: <MusicIcon />,
    expanded: ({ topInset }) => <Expanded media={media} topInset={topInset} look={settings} color={color} />,
    expandedSize: EXPANDED,
    theme,
  };
  return { activity, panel };
}
