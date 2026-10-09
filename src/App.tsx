import { useActivities } from "./activities";
import { Island } from "./components/Island";
import { useIslandHover } from "./hooks/useIslandHover";
import { useNotch } from "./hooks/useNotch";
import { usePlatform, useSettings } from "./hooks/useSettings";
import type { Notch, Platform, Settings } from "./lib/native";

function Surface({ notch, settings, platform }: { notch: Notch; settings: Settings; platform: Platform }) {
  // no modo clique o hover só serve pra saber quando o mouse saiu: sem delay
  const hovered = useIslandHover(settings.expandOn === "hover" ? settings.hoverDelayMs : 0);
  const [activity = null] = useActivities(settings);
  return <Island notch={notch} activity={activity} hovered={hovered} settings={settings} platform={platform} />;
}

export function App() {
  const notch = useNotch();
  const { settings } = useSettings();
  const platform = usePlatform();
  return notch && settings && platform ? <Surface notch={notch} settings={settings} platform={platform} /> : null;
}
