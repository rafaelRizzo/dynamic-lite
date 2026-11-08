import { AnimatePresence, motion } from "motion/react";
import { useEffect, useState } from "react";
import type { ShelfItem, Settings } from "../../lib/native";
import { FolderIcon } from "../icons";
import type { Panel } from "../../activities/types";
import {
  AddFileIcon,
  AudioFileIcon,
  CloseIcon,
  DoneCheck,
  DropFolder,
  GenericFileIcon,
  ImageFileIcon,
  OpenIcon,
  PasteImageIcon,
  RevealIcon,
  Wave,
} from "./Effects";
import { baseName, formatSize, presetById, PRESETS, type Preset } from "./presets";
import { MAX_BATCH, useShelf, type Job, type Shelf } from "./useShelf";

const WIDTH = 400;
/** cabeçalho + lista de 3 linhas + respiro */
const HEIGHT = 196;
/** a faixa de seleção em lote (contador + formatos) */
const BATCH_EXTRA = 62;
/** o visto "recém concluído" pisca o fundo da linha por este tempo */
const FLASH_MS = 2000;

const KIND_LABEL = { audio: "Áudio", image: "Imagem", file: "Arquivo" } as const;
const KIND_ICON = { audio: <AudioFileIcon />, image: <ImageFileIcon />, file: <GenericFileIcon /> } as const;

type RowProps = {
  item: ShelfItem;
  job: Job | undefined;
  expanded: boolean;
  normalize: boolean;
  onToggle: () => void;
  onNormalize: () => void;
  onConvert: (preset: Preset) => void;
  /** modo seleção: marcar vários pra converter juntos */
  select?: { on: boolean; disabled: boolean; toggle: () => void };
  shelf: Shelf;
};

function Status({ item, job, shelf }: { item: ShelfItem; job: Job | undefined; shelf: Shelf }) {
  if (job?.running) {
    const preset = presetById(job.preset);
    return (
      <div className="fx-sub">
        <Wave />
        <span>
          {preset?.label ?? "Convertendo"} · {Math.round(job.fraction * 100)}%
        </span>
      </div>
    );
  }
  if (job?.done) {
    return (
      <div className="fx-sub" data-ok>
        <span className="fx-sub-text">Salvo: {baseName(job.done.path)}</span>
        <button className="fx-inline" onClick={() => shelf.revealFile(job.done!.path)}>
          Mostrar
        </button>
      </div>
    );
  }
  if (job?.error) {
    return (
      <div className="fx-sub" data-bad data-tip={job.error}>
        {job.error}
      </div>
    );
  }
  return (
    <div className="fx-sub">
      {KIND_LABEL[item.kind]} · {formatSize(item.size)}
    </div>
  );
}

function Row({ item, job, expanded, normalize, onToggle, onNormalize, onConvert, select, shelf }: RowProps) {
  const fresh = !!job?.done && Date.now() - (job.finishedAt ?? 0) < FLASH_MS;
  const busy = !!job?.running;
  return (
    <motion.div
      layout="position"
      className="fx-row"
      data-flash={fresh || undefined}
      data-select={select ? "" : undefined}
      initial={{ opacity: 0, y: 6 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.96 }}
      transition={{ duration: 0.18 }}
    >
        {select && (item.kind === "audio" ? (
          <button
            className="fx-check"
            role="checkbox"
            aria-checked={select.on}
            data-on={select.on || undefined}
            disabled={busy || (!select.on && select.disabled)}
            onClick={select.toggle}
            aria-label="Selecionar"
          >
            <svg viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M3.5 8.5l3 3 6-7" />
            </svg>
          </button>
        ) : (
          <span />
        ))}
      <div className="fx-ico" data-kind={item.kind} data-done={job?.done ? "" : undefined}>
        {job?.done ? <DoneCheck key={job.finishedAt} /> : KIND_ICON[item.kind]}
      </div>
      <div className="fx-meta">
        <div className="fx-name" data-tip={item.name} data-tip-overflow>
          {item.name}
        </div>
        <Status item={item} job={job} shelf={shelf} />
      </div>
      <div className="fx-acts">
        {item.kind === "audio" && !select && (
          <button className="fx-conv" data-open={expanded || undefined} disabled={busy} onClick={onToggle}>
            Converter
          </button>
        )}
        <button className="fx-ib" data-tip="Abrir" aria-label="Abrir" onClick={() => shelf.open(item.id)}>
          <OpenIcon />
        </button>
        <button className="fx-ib" data-tip="Mostrar na pasta" aria-label="Mostrar na pasta" onClick={() => shelf.reveal(item.id)}>
          <RevealIcon />
        </button>
        <button className="fx-ib" data-tip="Remover da prateleira" aria-label="Remover da prateleira" onClick={() => shelf.remove(item.id)}>
          <CloseIcon />
        </button>
      </div>
      {busy && (
        <div className="fx-bar">
          <i style={{ width: `${Math.round((job?.fraction ?? 0) * 100)}%` }} />
        </div>
      )}
      {expanded && !busy && (
        <div className="fx-chips">
          {PRESETS.map((p) => (
            <button key={p.id} className="fx-chip" onClick={() => onConvert(p)}>
              {p.label}
            </button>
          ))}
          <button className="fx-chip" data-toggle aria-pressed={normalize} onClick={onNormalize} data-tip="Leva o volume ao máximo sem distorcer">
            Normalizar
          </button>
        </div>
      )}
    </motion.div>
  );
}

/** Rola a fileira de lado com a roda (ou o deslize do touchpad) e impede que o gesto troque de aba da ilha. */
function scrollSideways(el: HTMLDivElement | null) {
  if (!el) return;
  // as bordas esmaecem só do lado que ainda tem conteúdo escondido
  const fade = () => {
    el.style.setProperty("--ml", el.scrollLeft > 2 ? "28px" : "0px");
    el.style.setProperty("--mr", el.scrollLeft + el.clientWidth < el.scrollWidth - 2 ? "28px" : "0px");
  };
  fade();
  el.addEventListener("scroll", fade, { passive: true });
  el.addEventListener(
    "wheel",
    (e) => {
      e.stopPropagation();
      if (el.scrollWidth <= el.clientWidth) return;
      e.preventDefault();
      el.scrollLeft += Math.abs(e.deltaX) > Math.abs(e.deltaY) ? e.deltaX : e.deltaY;
    },
    { passive: false },
  );
}

type Picked = [Set<string> | null, (next: Set<string> | null | ((p: Set<string> | null) => Set<string> | null)) => void];

function Expanded({ shelf, topInset, selection }: { shelf: Shelf; topInset: number; selection: Picked }) {
  const { items, jobs, dragging, notice } = shelf;
  const [openRow, setOpenRow] = useState<string | null>(null);
  const [normalize, setNormalize] = useState(false);
  const [confirming, setConfirming] = useState(false);
  // seleção em lote: `null` = modo normal (o estado fica no hook da aba: a ilha cresce enquanto se seleciona)
  const [picked, setPicked] = selection;
  // "Limpar" apaga as cópias de tudo: pede um segundo clique e desiste sozinho
  useEffect(() => {
    if (!confirming) return;
    const timer = setTimeout(() => setConfirming(false), 3000);
    return () => clearTimeout(timer);
  }, [confirming]);

  // Ctrl+V (⌘V no Mac) com a ilha em foco: cola os arquivos copiados no explorador, ou a imagem copiada
  const { pasteImage } = shelf;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "v") {
        e.preventDefault();
        pasteImage();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [pasteImage]);

  const audios = items.filter((i) => i.kind === "audio");
  // arquivos removidos saem da seleção
  useEffect(() => {
    setPicked((p) => (p ? new Set([...p].filter((id) => items.some((i) => i.id === id))) : p));
  }, [items]);
  // sem pelo menos dois áudios não há o que selecionar em lote (ex.: depois de Limpar): sai do modo
  const canSelect = audios.length > 1;
  useEffect(() => {
    if (!canSelect) setPicked(null);
  }, [canSelect, setPicked]);
  const toggle = (id: string) =>
    setPicked((p) => {
      const next = new Set(p);
      if (!next.delete(id)) next.add(id);
      return next;
    });
  const chosen = items.filter((i) => picked?.has(i.id));
  const allChosen = chosen.length === Math.min(audios.length, MAX_BATCH);

  const total = items.reduce((sum, i) => sum + i.size, 0);
  return (
    <div className="fx-expanded" style={{ paddingTop: topInset }} data-drag={dragging}>
      <div className="fx-head">
        <span>{items.length > 0 ? `${items.length} ${items.length === 1 ? "item" : "itens"} · ${formatSize(total)}` : "Prateleira"}</span>
        <div className="fx-head-actions">
          {audios.length > 1 && (
            <button
              className="fx-link"
              data-active={picked ? "" : undefined}
              onClick={() => setPicked(picked ? null : new Set())}
              data-tip={`Converter vários de uma vez (até ${MAX_BATCH})`}
            >
              {picked ? "Cancelar" : "Selecionar"}
            </button>
          )}
          <button className="fx-link" onClick={shelf.pick} data-tip="Escolher arquivos (abre em Downloads)">
            <AddFileIcon />
            Adicionar
          </button>
          <button className="fx-link" onClick={shelf.pasteImage} data-tip="Cola os arquivos ou o print copiados (Ctrl+V)">
            <PasteImageIcon />
            Colar
          </button>
          {items.length > 0 && (
            <button
              className="fx-link"
              data-danger={confirming || undefined}
              onClick={() => (confirming ? (shelf.clear(), setConfirming(false)) : setConfirming(true))}
            >
              {confirming ? "Apagar tudo?" : "Limpar"}
            </button>
          )}
        </div>
      </div>

      {picked && (
        <div className="fx-batch">
          <div className="fx-batch-head">
            <span>{chosen.length === 0 ? "Marque os áudios" : `${chosen.length} de até ${MAX_BATCH} selecionados`}</span>
            <button
              className="fx-inline"
              onClick={() => setPicked(allChosen ? new Set() : new Set(audios.slice(0, MAX_BATCH).map((i) => i.id)))}
            >
              {allChosen ? "Nenhum" : "Todos"}
            </button>
          </div>
          {chosen.length > 0 && (
            <div className="fx-chips" ref={scrollSideways}>
              {PRESETS.map((p) => (
                <button
                  key={p.id}
                  className="fx-chip"
                  onClick={() => {
                    shelf.convertMany(chosen, p, normalize);
                    setPicked(null);
                  }}
                >
                  {p.label}
                </button>
              ))}
              <button className="fx-chip" data-toggle aria-pressed={normalize} onClick={() => setNormalize(!normalize)}>
                Normalizar
              </button>
            </div>
          )}
        </div>
      )}

      {items.length === 0 ? (
        <div className="fx-zone" data-drag={dragging || undefined}>
          <DropFolder />
          <div className="fx-zone-title">{dragging ? "Solte pra guardar" : "Solte arquivos aqui"}</div>
          <div className="fx-zone-sub">Áudios ganham conversão: WAV, GSM, µ-law, A-law, MP3</div>
        </div>
      ) : (
        <div className="fx-list">
          <AnimatePresence initial={false}>
            {items.map((item) => (
              <Row
                key={item.id}
                item={item}
                job={jobs[item.id]}
                expanded={openRow === item.id}
                normalize={normalize}
                onToggle={() => setOpenRow(openRow === item.id ? null : item.id)}
                onNormalize={() => setNormalize(!normalize)}
                onConvert={(preset) => {
                  setOpenRow(null);
                  shelf.convert(item, preset, normalize);
                }}
                select={picked ? { on: picked.has(item.id), disabled: picked.size >= MAX_BATCH, toggle: () => toggle(item.id) } : undefined}
                shelf={shelf}
              />
            ))}
          </AnimatePresence>
          {dragging && <div className="fx-drop-hint">Solte pra adicionar</div>}
        </div>
      )}

      <AnimatePresence>
        {notice && (
          <motion.div className="fx-toast" initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: 8 }} transition={{ duration: 0.16 }}>
            {notice}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

/** Aba Arquivos: prateleira (arquivos e prints guardados) e conversor de áudio. */
export function useFilesPanel(settings: Settings): Panel | null {
  const shelf = useShelf(settings);
  const selection = useState<Set<string> | null>(null);
  const picking = selection[0] !== null;
  if (!settings.showFiles) return null;
  return {
    id: "files",
    label: "Arquivos",
    icon: <FolderIcon />,
    expanded: ({ topInset }) => <Expanded shelf={shelf} topInset={topInset} selection={selection} />,
    expandedSize: { width: WIDTH, height: HEIGHT + (picking ? BATCH_EXTRA : 0) },
  };
}
