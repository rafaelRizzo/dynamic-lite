import { useEffect, useState } from "react";
import { native, subscribe } from "../lib/native";

/** Hover vindo do Rust (o webview não recebe mouse com click-through ligado). Entrada e saída com delay. */
export function useIslandHover(enterDelayMs = 100, exitDelayMs = 300): boolean {
  const [hovered, setHovered] = useState(false);
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const unsubscribe = subscribe(
      native.onHover((inside) => {
        // qualquer mudança cancela a pendente: voltar durante o delay de saída mantém aberta, sair antes de abrir não abre
        clearTimeout(timer);
        if (inside) timer = setTimeout(() => setHovered(true), enterDelayMs);
        else timer = setTimeout(() => setHovered(false), exitDelayMs);
      }),
    );
    return () => {
      clearTimeout(timer);
      unsubscribe();
    };
  }, [enterDelayMs, exitDelayMs]);
  return hovered;
}
