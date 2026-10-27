import type { Settings } from "../lib/native";
import { useAgendaPanel } from "../panels/agenda";
import { useClaudePanel } from "../panels/claude";
import { usePomodoroFeature } from "../panels/pomodoro";
import { useShortcutsPanel } from "../panels/shortcuts";
import { useNowPlayingFeature } from "./now-playing";
import { useSystemAudioFeature } from "./system-audio";
import { useUpdateFeature } from "./update";
import type { Activity, Panel } from "./types";

/**
 * Tudo que a Island pode mostrar. `activities` (Compact) por prioridade: cada lado do Notch fica com a primeira
 * que o preenche. `panels` (abas do Expanded) na ordem da barra. Nova feature = novo hook aqui.
 */
export function useIslandContent(settings: Settings, updateSettings: (patch: Partial<Settings>) => void): { activities: Activity[]; panels: Panel[] } {
  const nowPlaying = useNowPlayingFeature(settings);
  const systemAudio = useSystemAudioFeature(settings);
  const update = useUpdateFeature();
  const pomodoro = usePomodoroFeature(settings, updateSettings);
  const agenda = useAgendaPanel(settings);
  const claude = useClaudePanel(settings);
  const shortcuts = useShortcutsPanel(settings);
  return {
    activities: [pomodoro.activity, nowPlaying.activity, systemAudio.activity, update.activity].filter((a): a is Activity => a !== null),
    panels: [nowPlaying.panel, systemAudio.panel, agenda, pomodoro.panel, claude, shortcuts, update.panel].filter((p): p is Panel => p !== null),
  };
}
