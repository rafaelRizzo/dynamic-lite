import { useIslandContent } from "./activities";
import { useAlertPanel } from "./activities/alert";
import { Island } from "./components/Island";
import { useIslandHover } from "./hooks/useIslandHover";
import { useNotch } from "./hooks/useNotch";
import { usePlatform, useSettings } from "./hooks/useSettings";
import type { Notch, Platform, Settings } from "./lib/native";

type SurfaceProps = { notch: Notch; settings: Settings; update: (patch: Partial<Settings>) => void; platform: Platform };

function Surface({ notch, settings, update, platform }: SurfaceProps) {
  // no modo clique o hover só serve pra saber quando o mouse saiu: sem delay
  const hovered = useIslandHover(settings.expandOn === "hover" ? settings.hoverDelayMs : 0, settings.hoverExitDelayMs);
  const { activities, panels } = useIslandContent(settings, update);
  const alert = useAlertPanel(hovered);
  return (
    <Island notch={notch} activities={activities} panels={panels} forced={alert} hovered={hovered} settings={settings} platform={platform} />
  );
}

export function App() {
  const notch = useNotch();
  const { settings, update } = useSettings();
  const platform = usePlatform();
  return notch && settings && platform ? <Surface notch={notch} settings={settings} update={update} platform={platform} /> : null;
}
