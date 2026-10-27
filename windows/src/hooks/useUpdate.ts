import { useCallback, useEffect, useState } from "react";
import { native, subscribe, type UpdateInfo } from "../lib/native";

/** Atualização pendente (checada pelo Rust ao abrir e a cada 24h) e a instalação dela. */
export function useUpdate() {
  const [info, setInfo] = useState<UpdateInfo | null>(null);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    native.getUpdate().then(setInfo);
    const offAvailable = subscribe(native.onUpdate(setInfo));
    // a instalação pode ter começado pelo menu: todas as janelas mostram "Instalando…"
    const offInstalling = subscribe(native.onUpdateInstalling(() => setInstalling(true)));
    return () => {
      offAvailable();
      offInstalling();
    };
  }, []);

  const install = useCallback(() => {
    setInstalling(true);
    setError(null);
    native.installUpdate().catch((e) => {
      setInstalling(false);
      setError(String(e));
    });
  }, []);

  return { info, setInfo, installing, error, install };
}
