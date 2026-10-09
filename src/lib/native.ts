import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Notch = { width: number; height: number; hasNotch: boolean };

export type PlayerStatus = "closed" | "stopped" | "playing" | "paused";

export type Track = {
  id: string;
  name: string;
  artist: string;
  album: string;
  artworkUrl: string;
  /** segundos */
  duration: number;
};

export type PlayerState = {
  status: PlayerStatus;
  track: Track | null;
  /** segundos, no momento da leitura */
  position: number;
  /** 0-100 */
  volume: number;
};

export type SpotifyAction = "playpause" | "next" | "previous";

const on =
  <T,>(event: string) =>
  (cb: (payload: T) => void): Promise<UnlistenFn> =>
    listen<T>(event, (e) => cb(e.payload));

/** Ponte com o backend Rust: comandos e eventos. */
export const native = {
  getNotch: () => invoke<Notch>("get_notch"),
  setIslandSize: (width: number, height: number) => invoke<void>("set_island_size", { width, height }),
  spotifyState: () => invoke<PlayerState>("spotify_state"),
  spotifyControl: (action: SpotifyAction) => invoke<void>("spotify_control", { action }),
  spotifySeek: (position: number) => invoke<void>("spotify_seek", { position }),
  spotifyVolume: (volume: number) => invoke<void>("spotify_volume", { volume }),
  onNotch: on<Notch>("island://notch"),
  onHover: on<boolean>("island://hover"),
  onSpotify: on<PlayerState>("spotify://state"),
};

/** Assina um evento no useEffect sem vazar listener se o componente desmontar antes do `listen` resolver. */
export function subscribe(pending: Promise<UnlistenFn>): () => void {
  let unlisten: UnlistenFn | undefined;
  let disposed = false;
  pending.then((fn) => (disposed ? fn() : (unlisten = fn)));
  return () => {
    disposed = true;
    unlisten?.();
  };
}
