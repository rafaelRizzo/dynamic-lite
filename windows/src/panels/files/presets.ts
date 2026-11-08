import type { ConvertOptions } from "../../lib/native";

export type Preset = {
  id: string;
  label: string;
  /** vai no nome do arquivo gerado: `musica_8k.wav` */
  suffix: string;
  options: Omit<ConvertOptions, "normalize">;
};

/** Formatos de saída do conversor. Os de telefonia (G.711, GSM) já saem em 8 kHz mono. */
export const PRESETS: Preset[] = [
  { id: "wav8", label: "WAV 8 kHz mono", suffix: "8k", options: { format: "wav", sampleRate: 8000, channels: 1 } },
  { id: "wav16", label: "WAV 16 kHz mono", suffix: "16k", options: { format: "wav", sampleRate: 16000, channels: 1 } },
  { id: "wav44", label: "WAV 44,1 kHz", suffix: "44k", options: { format: "wav", sampleRate: 44100, channels: 2 } },
  { id: "ulaw", label: "µ-law", suffix: "ulaw", options: { format: "wav-ulaw" } },
  { id: "alaw", label: "A-law", suffix: "alaw", options: { format: "wav-alaw" } },
  { id: "gsm", label: "GSM", suffix: "gsm", options: { format: "gsm" } },
  { id: "wav49", label: "GSM em WAV", suffix: "wav49", options: { format: "wav-gsm" } },
  { id: "mp3-128", label: "MP3 128k", suffix: "128k", options: { format: "mp3", bitrate: 128 } },
  { id: "mp3-192", label: "MP3 192k", suffix: "192k", options: { format: "mp3", bitrate: 192 } },
];

export const presetById = (id: string | undefined) => PRESETS.find((p) => p.id === id);

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let i = 0;
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024;
    i++;
  }
  return `${value >= 100 ? Math.round(value) : value.toFixed(1).replace(".", ",")} ${units[i]}`;
}

export const baseName = (path: string) => path.split(/[\\/]/).pop() ?? path;
