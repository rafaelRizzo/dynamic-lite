import { useCallback, useEffect, useState } from "react";
import { native, subscribe, type PlayerState } from "../../lib/native";

export type Playback = { state: PlayerState; receivedAt: number };

const EMPTY: Playback = { state: { status: "closed", track: null, position: 0, volume: 0 }, receivedAt: 0 };

/** Posição atual interpolada desde a última leitura do Spotify. */
export function positionAt({ state, receivedAt }: Playback, now: number): number {
  const elapsed = state.status === "playing" ? (now - receivedAt) / 1000 : 0;
  return Math.min(state.position + elapsed, state.track?.duration ?? 0);
}

export function useNowPlaying() {
  const [playback, setPlayback] = useState<Playback>(EMPTY);

  useEffect(() => {
    const receive = (state: PlayerState) => setPlayback({ state, receivedAt: Date.now() });
    native.spotifyState().then(receive);
    return subscribe(native.onSpotify(receive));
  }, []);

  // atualizações otimistas: a confirmação chega pelo evento do watcher
  const toggle = useCallback(() => {
    setPlayback((p) => {
      const now = Date.now();
      const status = p.state.status === "playing" ? "paused" : "playing";
      return { state: { ...p.state, status, position: positionAt(p, now) }, receivedAt: now };
    });
    native.spotifyControl("playpause");
  }, []);

  const seek = useCallback((position: number) => {
    setPlayback((p) => ({ state: { ...p.state, position }, receivedAt: Date.now() }));
    native.spotifySeek(position);
  }, []);

  const setVolume = useCallback((volume: number) => {
    const v = Math.round(Math.min(100, Math.max(0, volume)));
    setPlayback((p) => ({ ...p, state: { ...p.state, volume: v } }));
    native.spotifyVolume(v);
  }, []);

  const next = useCallback(() => native.spotifyControl("next"), []);
  const previous = useCallback(() => native.spotifyControl("previous"), []);

  return { playback, toggle, seek, setVolume, next, previous };
}

export type NowPlayingControls = ReturnType<typeof useNowPlaying>;
