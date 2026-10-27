import { useCallback, useEffect, useRef, useState } from "react";
import { native, subscribe, type Platform, type Settings } from "../lib/native";

/** Settings sincronizadas entre janelas: o Rust é a fonte da verdade e avisa cada mudança. */
export function useSettings() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const current = useRef<Settings | null>(null);
  current.current = settings;

  useEffect(() => {
    native.getSettings().then(setSettings);
    return subscribe(native.onSettings(setSettings));
  }, []);

  const update = useCallback((patch: Partial<Settings>) => {
    if (!current.current) return;
    const next = { ...current.current, ...patch };
    current.current = next;
    setSettings(next);
    // o Rust recusou (ex.: valor que ele não conhece): volta pro que está salvo, em vez de fingir que aplicou
    native.setSettings(next).catch((e) => {
      console.error("set_settings falhou:", e);
      native.getSettings().then(setSettings);
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
