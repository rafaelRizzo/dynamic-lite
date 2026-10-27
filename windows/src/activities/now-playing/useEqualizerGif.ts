import { useEffect, useState } from "react";
import { native } from "../../lib/native";

/** Um GIF por vez: compacta e aberta compartilham a mesma URL em vez de ler 50 MB duas vezes. */
let cached: { rev: number; url: Promise<string> } | null = null;

function gifUrl(rev: number): Promise<string> {
  if (cached?.rev === rev) return cached.url;
  cached?.url.then(URL.revokeObjectURL);
  const url = native.equalizerGifRead().then((bytes) => URL.createObjectURL(new Blob([bytes], { type: "image/gif" })));
  cached = { rev, url };
  return url;
}

/** URL do GIF salvo; `null` sem GIF (`rev` 0) ou enquanto carrega. */
export function useEqualizerGif(rev: number): string | null {
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    if (!rev) {
      setUrl(null);
      return;
    }
    let alive = true;
    gifUrl(rev)
      .then((u) => alive && setUrl(u))
      .catch(() => alive && setUrl(null));
    return () => {
      alive = false;
    };
  }, [rev]);
  return url;
}
