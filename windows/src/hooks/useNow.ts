import { useEffect, useState } from "react";

/** Relógio que re-renderiza a cada `intervalMs` enquanto montado. */
export function useNow(intervalMs: number): number {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    // o intervalo mudou (ex.: timer parado -> rodando): não espera o próximo tick pra ter a hora certa
    setNow(Date.now());
    const id = setInterval(() => setNow(Date.now()), intervalMs);
    return () => clearInterval(id);
  }, [intervalMs]);
  return now;
}
