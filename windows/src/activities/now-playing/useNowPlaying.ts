import { useCallback, useEffect, useRef, useState } from "react";
import { native, subscribe, type PlayerState, type Repeat } from "../../lib/native";

export type Playback = { state: PlayerState; receivedAt: number };

/** Volume restaurado ao desmutar quando não há um anterior conhecido. */
const DEFAULT_VOLUME = 50;

/** Após mexer no volume, leituras do Spotify (ainda com o valor antigo) não sobrescrevem o valor local. */
const VOLUME_HOLD_MS = 1500;

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
  // comandos de volume em série (osascript concorrentes terminam fora de ordem), só o último valor importa
  const volumeHold = useRef(0);
  const volumeSend = useRef<{ busy: boolean; next: number | null }>({ busy: false, next: null });

  useEffect(() => {
    const receive = (incoming: PlayerState) =>
      setPlayback((p) => {
        const held = Date.now() < volumeHold.current;
        const state = held ? { ...incoming, volume: p.state.volume } : incoming;
        return { state, receivedAt: Date.now() };
      });
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
    volumeHold.current = Date.now() + VOLUME_HOLD_MS;
    setPlayback((p) => ({ ...p, state: { ...p.state, volume: v } }));
    const send = volumeSend.current;
    if (send.busy) {
      send.next = v;
      return;
    }
    send.busy = true;
    const run = (value: number) =>
      native
        .spotifyVolume(value)
        .catch(() => {})
        .finally(() => {
          const pending = send.next;
          send.next = null;
          if (pending === null) send.busy = false;
          else run(pending);
        });
    run(v);
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
