import { useEffect, useState } from "react";
import { native, subscribe } from "../lib/native";

/** Hover vindo do Rust (o webview não recebe mouse com click-through ligado). Entrada com delay, saída imediata. */
export function useIslandHover(enterDelayMs = 100): boolean {
  const [hovered, setHovered] = useState(false);
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const unsubscribe = subscribe(
      native.onHover((inside) => {
        clearTimeout(timer);
        if (inside) timer = setTimeout(() => setHovered(true), enterDelayMs);
        else setHovered(false);
      }),
    );
    return () => {
      clearTimeout(timer);
      unsubscribe();
    };
  }, [enterDelayMs]);
  return hovered;
}
