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
  shuffle: boolean;
  /** `track` (repetir a faixa) é emulado pelo app; o Spotify só tem repetir liga/desliga */
  repeat: Repeat;
};

export type Repeat = "off" | "context" | "track";

export type Style = "black" | "translucent" | "glass";

export type Settings = {
  style: Style;
  /** 0.6-1, só no Style Translucent */
  opacity: number;
  expandOn: "hover" | "click";
  hoverDelayMs: number;
  hideIdleWithoutNotch: boolean;
  accent: "artwork" | "white";
  haptics: boolean;
  showMenuBarIcon: boolean;
  /** "auto", "main" ou o nome da tela */
  display: string;
  allSpaces: boolean;
  hideInMissionControl: boolean;
};

export type Platform = { glassSupported: boolean };

/** Forma da Island centrada no topo da janela, Ears incluídas. */
export type IslandShape = { width: number; height: number; radius: number; ear: number };

export type SpotifyAction = "playpause" | "next" | "previous";

const on =
  <T,>(event: string) =>
  (cb: (payload: T) => void): Promise<UnlistenFn> =>
    listen<T>(event, (e) => cb(e.payload));

/** Ponte com o backend Rust: comandos e eventos. */
export const native = {
  getNotch: () => invoke<Notch>("get_notch"),
  setIslandSize: (width: number, height: number) => invoke<void>("set_island_size", { width, height }),
  getPlatform: () => invoke<Platform>("get_platform"),
  listDisplays: () => invoke<string[]>("list_displays"),
  /** `target` null recolhe o vidro de volta pra `rest` (Compact/Idle) e esconde. */
  setGlass: (target: IslandShape | null, rest: IslandShape) => invoke<void>("set_glass", { target, rest }),
  haptic: () => invoke<void>("haptic"),
  showContextMenu: () => invoke<void>("show_context_menu"),
  getSettings: () => invoke<Settings>("get_settings"),
  setSettings: (value: Settings) => invoke<void>("set_settings", { value }),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => invoke<boolean>("set_autostart", { enabled }),
  spotifyState: () => invoke<PlayerState>("spotify_state"),
  spotifyControl: (action: SpotifyAction) => invoke<void>("spotify_control", { action }),
  spotifySeek: (position: number) => invoke<void>("spotify_seek", { position }),
  spotifyVolume: (volume: number) => invoke<void>("spotify_volume", { volume }),
  spotifyShuffle: (enabled: boolean) => invoke<void>("spotify_shuffle", { enabled }),
  spotifyRepeat: (mode: Repeat) => invoke<void>("spotify_repeat", { mode }),
  /** Traz o Spotify pra frente na faixa (`spotify:track:...`), sem mexer na reprodução. */
  spotifyOpen: (uri: string) => invoke<void>("spotify_open", { uri }),
  onNotch: on<Notch>("island://notch"),
  onHover: on<boolean>("island://hover"),
  onSpotify: on<PlayerState>("spotify://state"),
  onSettings: on<Settings>("settings://changed"),
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
