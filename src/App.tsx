import { useActivities } from "./activities";
import { Island } from "./components/Island";
import { useIslandHover } from "./hooks/useIslandHover";
import { useNotch } from "./hooks/useNotch";
import type { Notch } from "./lib/native";

function Surface({ notch }: { notch: Notch }) {
  const hovered = useIslandHover();
  const [activity = null] = useActivities(notch);
  return <Island notch={notch} activity={activity} hovered={hovered} />;
}

export function App() {
  const notch = useNotch();
  return notch ? <Surface notch={notch} /> : null;
}
