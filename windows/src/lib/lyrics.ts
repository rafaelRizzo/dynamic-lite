export type LyricLine = { time: number; text: string };

const STAMP = /\[(\d+):(\d{1,2})(?:[.:](\d{1,3}))?\]/g;

/** Texto LRC (`[mm:ss.xx] linha`) em linhas com o tempo em segundos, em ordem. Metadados (`[ar:...]`) e linhas sem tempo são ignorados. */
export function parseLrc(lrc: string): LyricLine[] {
  const lines: LyricLine[] = [];
  for (const raw of lrc.split(/\r?\n/)) {
    const stamps = [...raw.matchAll(STAMP)];
    if (stamps.length === 0) continue;
    const text = raw.replace(STAMP, "").trim();
    for (const [, min, sec, frac = ""] of stamps) {
      // fração tem 1-3 dígitos: ".5" = 500 ms, ".50" = 500 ms, ".500" = 500 ms
      const time = Number(min) * 60 + Number(sec) + (frac ? Number(frac) / 10 ** frac.length : 0);
      lines.push({ time, text });
    }
  }
  return lines.sort((a, b) => a.time - b.time);
}

/** Índice da linha tocando em `position` (s): a última que já começou, ou -1 antes da primeira. */
export function lineAt(lines: LyricLine[], position: number): number {
  let lo = 0;
  let hi = lines.length - 1;
  let found = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (lines[mid].time <= position) {
      found = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return found;
}
