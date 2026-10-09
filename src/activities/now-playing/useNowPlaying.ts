import { useCallback, useEffect, useRef, useState } from "react";
import { native, subscribe, type PlayerState, type Repeat } from "../../lib/native";

export type Playback = { state: PlayerState; receivedAt: number };

/** Volume restaurado ao desmutar quando não há um anterior conhecido. */
const DEFAULT_VOLUME = 50;

/** Mesma ordem do botão do Spotify: desligado → repetir tudo → repetir faixa. */
const NEXT_REPEAT: Record<Repeat, Repeat> = { off: "context", context: "track", track: "off" };

const EMPTY: Playback = { state: { status: "closed", track: null, position: 0, volume: 0, shuffle: false, repeat: "off" }, receivedAt: 0 };

/** Posição atual interpolada desde a última leitura do Spotify. */
export function positionAt({ state, receivedAt }: Playback, now: number): number {
  const elapsed = state.status === "playing" ? (now - receivedAt) / 1000 : 0;
  return Math.min(state.position + elapsed, state.track?.duration ?? 0);
}

export function useNowPlaying() {
  const [playback, setPlayback] = useState<Playback>(EMPTY);
  // estado atual pra callbacks estáveis; efeitos (comandos ao Spotify) nunca dentro do setState
  const stateRef = useRef(playback.state);
  stateRef.current = playback.state;
  // o Spotify não tem mute: mutar é volume 0, lembrando o anterior pra voltar
  const beforeMute = useRef(DEFAULT_VOLUME);

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
    if (v > 0) beforeMute.current = v;
    setPlayback((p) => ({ ...p, state: { ...p.state, volume: v } }));
    native.spotifyVolume(v);
  }, []);

  const toggleMute = useCallback(() => {
    setVolume(stateRef.current.volume > 0 ? 0 : beforeMute.current);
  }, [setVolume]);

  const toggleShuffle = useCallback(() => {
    const shuffle = !stateRef.current.shuffle;
    setPlayback((p) => ({ ...p, state: { ...p.state, shuffle } }));
    native.spotifyShuffle(shuffle);
  }, []);

  const cycleRepeat = useCallback(() => {
    const repeat = NEXT_REPEAT[stateRef.current.repeat];
    setPlayback((p) => ({ ...p, state: { ...p.state, repeat } }));
    native.spotifyRepeat(repeat);
  }, []);

  const next = useCallback(() => native.spotifyControl("next"), []);
  const previous = useCallback(() => native.spotifyControl("previous"), []);

  return { playback, toggle, seek, setVolume, toggleMute, toggleShuffle, cycleRepeat, next, previous };
}

export type NowPlayingControls = ReturnType<typeof useNowPlaying>;
