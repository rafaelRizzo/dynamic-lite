import type { Settings } from "../lib/native";
import { useNowPlayingActivity } from "./now-playing";
import type { Activity } from "./types";

/** Activities ativas por prioridade; a Island mostra a primeira. Nova Activity = novo hook aqui. */
export function useActivities(settings: Settings): Activity[] {
  const nowPlaying = useNowPlayingActivity(settings);
  return [nowPlaying].filter((a): a is Activity => a !== null);
}
