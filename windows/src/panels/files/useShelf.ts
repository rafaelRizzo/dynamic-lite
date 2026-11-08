import { useCallback, useEffect, useRef, useState } from "react";
import { native, subscribe, type Converted, type Settings, type ShelfItem } from "../../lib/native";
import type { Preset } from "./presets";

/** Pede à Island pra abrir uma aba (a detalhe é o id). Usado ao arrastar arquivos por cima dela. */
export const OPEN_TAB_EVENT = "island:tab";
/** Pede à Island pra ficar aberta (detalhe `true`) ou liberar (`false`), mesmo com o mouse fora dela. */
export const HOLD_EVENT = "island:hold";
const hold = (on: boolean) => window.dispatchEvent(new CustomEvent(HOLD_EVENT, { detail: on }));

export type Job = {
  running: boolean;
  /** 0-1 */
  fraction: number;
  preset: string;
  done?: Converted;
  finishedAt?: number;
  error?: string;
};

const NOTICE_MS = 3500;
/** Máximo de arquivos convertidos de uma vez. */
export const MAX_BATCH = 20;

export const errorText = (e: unknown) => (typeof e === "string" ? e : e instanceof Error ? e.message : "Algo deu errado");

/** Prateleira de arquivos e conversões. Fica no hook da aba (não no painel) pra sobreviver à troca de abas. */
export function useShelf(settings: Settings) {
  const enabled = settings.showFiles;
  const [items, setItems] = useState<ShelfItem[]>([]);
  const [jobs, setJobs] = useState<Record<string, Job>>({});
  const [dragging, setDragging] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const noticeTimer = useRef<number | undefined>(undefined);
  // o som e o volume mais recentes sem recriar os callbacks
  const settingsRef = useRef(settings);
  settingsRef.current = settings;

  const flash = useCallback((text: string) => {
    setNotice(text);
    clearTimeout(noticeTimer.current);
    noticeTimer.current = window.setTimeout(() => setNotice(null), NOTICE_MS);
  }, []);
  useEffect(() => () => clearTimeout(noticeTimer.current), []);

  useEffect(() => {
    if (enabled) native.shelfList().then(setItems).catch(() => {});
  }, [enabled]);

  // soltar arquivos do sistema: o Rust trata o arrasto (janela sem foco, Island fechada) e avisa por eventos
  useEffect(() => {
    if (!enabled) return;
    const offs = [
      native.onShelfDrag((dragging) => {
        setDragging(dragging);
        if (dragging) window.dispatchEvent(new CustomEvent(OPEN_TAB_EVENT, { detail: "files" }));
      }),
      native.onShelfChanged(setItems),
      native.onShelfError(flash),
    ].map(subscribe);
    return () => offs.forEach((off) => off());
  }, [enabled, flash]);

  // a Island fechada ignora o mouse e não recebe o arrasto: o Rust avisa quando algo é arrastado pra cima dela
  useEffect(() => {
    if (!enabled) return;
    return subscribe(native.onDragIn(() => window.dispatchEvent(new CustomEvent(OPEN_TAB_EVENT, { detail: "files" }))));
  }, [enabled]);

  useEffect(() => {
    if (!enabled) return;
    return subscribe(
      native.onConvertProgress(({ job, fraction }) =>
        setJobs((j) => (j[job]?.running ? { ...j, [job]: { ...j[job], fraction } } : j)),
      ),
    );
  }, [enabled]);

  const pasteImage = useCallback(() => native.shelfPasteImage().then(setItems).catch((e) => flash(errorText(e))), [flash]);

  // o seletor do sistema tira o mouse da ilha: segura ela aberta enquanto ele está na tela e um instante depois
  const pick = useCallback(() => {
    hold(true);
    return native
      .shelfPick()
      .then(setItems)
      .catch((e) => flash(errorText(e)))
      .finally(() => setTimeout(() => hold(false), 1500));
  }, [flash]);

  const remove = useCallback(
    (id: string) => {
      setJobs(({ [id]: _gone, ...rest }) => rest);
      native.shelfRemove(id).then(setItems).catch((e) => flash(errorText(e)));
    },
    [flash],
  );

  const clear = useCallback(() => {
    setJobs({});
    native.shelfClear().then(setItems).catch((e) => flash(errorText(e)));
  }, [flash]);

  const open = useCallback((id: string) => native.shelfOpen(id).catch((e) => flash(errorText(e))), [flash]);
  const reveal = useCallback((id: string) => native.shelfReveal(id).catch((e) => flash(errorText(e))), [flash]);
  const revealFile = useCallback((path: string) => native.revealFile(path).catch((e) => flash(errorText(e))), [flash]);

  /** Converte um arquivo; `sound: false` deixa o som pro fim de um lote. */
  const run = useCallback((item: ShelfItem, preset: Preset, normalize: boolean, sound: boolean) => {
    setJobs((j) => ({ ...j, [item.id]: { running: true, fraction: 0, preset: preset.id } }));
    return native
      .convertAudio(item.id, item.path, item.outDir, preset.suffix, { ...preset.options, normalize })
      .then((done) => {
        setJobs((j) => ({ ...j, [item.id]: { running: false, fraction: 1, preset: preset.id, done, finishedAt: Date.now() } }));
        const s = settingsRef.current;
        if (sound && s.convertSound) native.playSound(s.sfx, s.sfxVolume).catch(() => {});
        return true;
      })
      .catch((e) => {
        setJobs((j) => ({ ...j, [item.id]: { running: false, fraction: 0, preset: preset.id, error: errorText(e) } }));
        return false;
      });
  }, []);

  const convert = useCallback((item: ShelfItem, preset: Preset, normalize: boolean) => void run(item, preset, normalize, true), [run]);

  /** Vários de uma vez (até MAX_BATCH), um depois do outro; o som toca uma vez, no fim. */
  const convertMany = useCallback(
    async (list: ShelfItem[], preset: Preset, normalize: boolean) => {
      const batch = list.slice(0, MAX_BATCH);
      let ok = 0;
      for (const item of batch) if (await run(item, preset, normalize, false)) ok++;
      const s = settingsRef.current;
      if (ok > 0 && s.convertSound) native.playSound(s.sfx, s.sfxVolume).catch(() => {});
      flash(ok === batch.length ? `${ok} convertidos` : `${ok} de ${batch.length} convertidos`);
    },
    [run, flash],
  );

  return { items, jobs, dragging, notice, pasteImage, pick, remove, clear, open, reveal, revealFile, convert, convertMany };
}

export type Shelf = ReturnType<typeof useShelf>;
