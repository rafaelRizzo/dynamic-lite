import { useEffect, useState } from "react";
import { native, subscribe, type Notch } from "../lib/native";

export function useNotch(): Notch | null {
  const [notch, setNotch] = useState<Notch | null>(null);
  useEffect(() => {
    native.getNotch().then(setNotch);
    return subscribe(native.onNotch(setNotch));
  }, []);
  return notch;
}
