import { useCallback, useEffect, useState } from "react";
import { native, subscribe, type Platform, type Settings } from "../lib/native";

/** Settings sincronizadas entre janelas: o Rust é a fonte da verdade e avisa cada mudança. */
export function useSettings() {
  const [settings, setSettings] = useState<Settings | null>(null);

  useEffect(() => {
    native.getSettings().then(setSettings);
    return subscribe(native.onSettings(setSettings));
  }, []);

  const update = useCallback((patch: Partial<Settings>) => {
    setSettings((current) => {
      if (!current) return current;
      const next = { ...current, ...patch };
      native.setSettings(next);
      return next;
    });
  }, []);

  return { settings, update };
}

export function usePlatform(): Platform | null {
  const [platform, setPlatform] = useState<Platform | null>(null);
  useEffect(() => {
    native.getPlatform().then(setPlatform);
  }, []);
  return platform;
}
