import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

/** Espera antes da primeira dica; depois de uma aberta, as vizinhas aparecem na hora. */
const DELAY_MS = 450;
const GRACE_MS = 500;
/** Distância do alvo e folga mínima da borda da janela. */
const GAP = 8;
const MARGIN = 6;

type Side = "top" | "bottom";
type Target = { el: HTMLElement; text: string };
type Pos = { left: number; top: number; side: Side; glide: boolean };

/** Texto da dica de um elemento; `data-tip-overflow` só mostra quando o texto está cortado (reticências). */
function textOf(el: HTMLElement): string {
  if (el.dataset.tipOverflow !== undefined && el.scrollWidth <= el.clientWidth) return "";
  return el.dataset.tip ?? "";
}

/**
 * Dica de ferramenta da Island, no lugar do `title` nativo (lento, com cara de sistema e fora do tema).
 * Uma camada só, por delegação: qualquer elemento com `data-tip="texto"` ganha dica; `data-tip-side="bottom"`
 * abre embaixo (padrão: em cima). Fica sempre dentro da janela, com flip e clamp nas bordas.
 */
export function TipLayer() {
  const [target, setTarget] = useState<Target | null>(null);
  const [open, setOpen] = useState(false);
  const [pos, setPos] = useState<Pos | null>(null);
  const ref = useRef<HTMLDivElement>(null);
  const openRef = useRef(false);
  openRef.current = open;
  /** `open` do commit anterior: no commit que troca o alvo, diz se a dica já estava aberta */
  const wasOpen = useRef(false);

  useEffect(() => {
    let current: HTMLElement | null = null;
    let timer = 0;
    let lastHide = 0;

    const hide = () => {
      clearTimeout(timer);
      current = null;
      if (openRef.current) lastHide = Date.now();
      setOpen(false);
    };
    const show = (el: HTMLElement) => {
      const text = textOf(el);
      if (!text) return;
      setTarget({ el, text });
      setOpen(true);
    };
    const onOver = (e: PointerEvent) => {
      const el = e.target instanceof Element ? e.target.closest<HTMLElement>("[data-tip]") : null;
      if (el === current) return;
      clearTimeout(timer);
      if (!el) return hide();
      current = el;
      if (openRef.current) return show(el);
      timer = window.setTimeout(() => show(el), Date.now() - lastHide < GRACE_MS ? 0 : DELAY_MS);
    };
    // o mouse saiu da janela
    const onOut = (e: PointerEvent) => {
      if (!e.relatedTarget) hide();
    };

    document.addEventListener("pointerover", onOver);
    document.addEventListener("pointerout", onOut);
    document.addEventListener("pointerdown", hide, true);
    window.addEventListener("blur", hide);
    return () => {
      clearTimeout(timer);
      document.removeEventListener("pointerover", onOver);
      document.removeEventListener("pointerout", onOut);
      document.removeEventListener("pointerdown", hide, true);
      window.removeEventListener("blur", hide);
    };
  }, []);

  // enquanto aberta: acompanha troca de texto (toggles) e some se o alvo saiu do DOM (troca de aba)
  useEffect(() => {
    if (!open || !target) return;
    const { el } = target;
    const sync = () => {
      const text = textOf(el);
      if (!el.isConnected || !text) setOpen(false);
      else setTarget((t) => (t && t.el === el && t.text !== text ? { el, text } : t));
    };
    const observer = new MutationObserver(sync);
    observer.observe(el, { attributes: true, attributeFilter: ["data-tip"] });
    const alive = window.setInterval(sync, 250);
    return () => {
      observer.disconnect();
      clearInterval(alive);
    };
  }, [open, target]);

  // mede a dica já renderizada e posiciona antes do paint
  useLayoutEffect(() => {
    const tip = ref.current;
    if (!target || !tip) return;
    const r = target.el.getBoundingClientRect();
    // offset*: ignora o scale da animação de entrada
    const { offsetWidth: width, offsetHeight: height } = tip;
    const prefer: Side = target.el.dataset.tipSide === "bottom" ? "bottom" : "top";
    const fits = (s: Side) => (s === "top" ? r.top - GAP - height >= MARGIN : r.bottom + GAP + height <= innerHeight - MARGIN);
    const side: Side = fits(prefer) ? prefer : fits(prefer === "top" ? "bottom" : "top") ? (prefer === "top" ? "bottom" : "top") : prefer;
    const left = Math.min(Math.max(r.left + r.width / 2 - width / 2, MARGIN), Math.max(MARGIN, innerWidth - width - MARGIN));
    const top = side === "top" ? Math.max(MARGIN, r.top - GAP - height) : Math.min(r.bottom + GAP, innerHeight - height - MARGIN);
    // glide: já estava aberta e só trocou de alvo, então desliza em vez de teleportar
    setPos({ left, top, side, glide: wasOpen.current });
  }, [target]);
  useLayoutEffect(() => {
    wasOpen.current = open;
  }, [open]);

  return createPortal(
    <div
      ref={ref}
      className="tip"
      role="tooltip"
      data-open={open && pos !== null}
      data-side={pos?.side ?? "top"}
      data-glide={pos?.glide ?? false}
      style={{ left: pos?.left ?? 0, top: pos?.top ?? 0 }}
    >
      {target?.text}
    </div>,
    document.body,
  );
}
