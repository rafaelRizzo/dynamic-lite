import { useEffect, useRef, useState, type ReactNode } from "react";
import { formatDuration, parseDuration } from "../lib/duration";

export function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="st-section">
      <h2>{title}</h2>
      <div className="st-group">{children}</div>
    </section>
  );
}

export function Row({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="st-row">
      <div className="st-label">
        <span>{label}</span>
        {hint && <small>{hint}</small>}
      </div>
      <div className="st-control">{children}</div>
    </div>
  );
}

type Option<T extends string> = { value: T; label: string; disabled?: boolean; title?: string };

export function Segmented<T extends string>({ value, options, onChange }: { value: T; options: Option<T>[]; onChange: (v: T) => void }) {
  return (
    <div className="st-segmented" role="radiogroup">
      {options.map((o) => (
        <button
          key={o.value}
          role="radio"
          aria-checked={o.value === value}
          disabled={o.disabled}
          title={o.title}
          onClick={() => onChange(o.value)}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function Toggle({ checked, onChange, disabled }: { checked: boolean; onChange: (v: boolean) => void; disabled?: boolean }) {
  return (
    <button
      className="st-toggle"
      role="switch"
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onChange(!checked)}
    >
      <span />
    </button>
  );
}

type RangeProps = { value: number; min: number; max: number; step: number; format: (v: number) => string; onChange: (v: number) => void };

export function Range({ value, min, max, step, format, onChange }: RangeProps) {
  const pct = ((value - min) / (max - min)) * 100;
  return (
    <div className="st-range">
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        style={{ "--pct": `${pct}%` } as React.CSSProperties}
        onChange={(e) => onChange(Number(e.target.value))}
      />
      <output>{format(value)}</output>
    </div>
  );
}

export function Note({ children }: { children: ReactNode }) {
  return <p className="st-note">{children}</p>;
}

type ChipsProps = { options: { value: string; label: string }[]; selected: string[]; max: number; onChange: (v: string[]) => void };

/** Seleção múltipla ordenada pela ordem de escolha; passa do `max`, o resto fica desabilitado. */
export function Chips({ options, selected, max, onChange }: ChipsProps) {
  return (
    <div className="st-chips">
      {options.map((o) => {
        const on = selected.includes(o.value);
        return (
          <button
            key={o.value}
            className="st-chip"
            aria-pressed={on}
            disabled={!on && selected.length >= max}
            onClick={() => onChange(on ? selected.filter((v) => v !== o.value) : [...selected, o.value])}
          >
            {o.label}
          </button>
        );
      })}
    </div>
  );
}

/** Duração em minutos digitada ("25", "90m", "1h30", "1:30"), de 1 min a 24 h. Inválido volta pro valor anterior. */
export function DurationField({ value, onChange }: { value: number; onChange: (minutes: number) => void }) {
  const [text, setText] = useState<string | null>(null);
  const commit = () => {
    const parsed = text === null ? null : parseDuration(text);
    if (parsed !== null && parsed !== value) onChange(parsed);
    setText(null);
  };
  return (
    <input
      className="st-input"
      value={text ?? formatDuration(value)}
      size={7}
      aria-label="Duração, de 1 minuto a 24 horas"
      title="1 min a 24 h: 25, 90m, 1h30"
      onFocus={(e) => {
        setText(String(value));
        e.currentTarget.select();
      }}
      onChange={(e) => setText(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") e.currentTarget.blur();
        if (e.key === "Escape") {
          setText(null);
          e.currentTarget.blur();
        }
      }}
    />
  );
}

type SelectOption = { value: string; label: string; disabled?: boolean };

/** Select próprio: o `<select>` nativo do Windows é um retângulo quadrado que destoa do resto dos Ajustes. */
export function Select({ value, options, onChange }: { value: string; options: SelectOption[]; onChange: (v: string) => void }) {
  const [open, setOpen] = useState(false);
  const [up, setUp] = useState(false);
  const [active, setActive] = useState(-1);
  const root = useRef<HTMLDivElement>(null);
  const list = useRef<HTMLUListElement>(null);
  const current = options.find((o) => o.value === value);

  const show = () => {
    const box = root.current?.getBoundingClientRect();
    // abre pra cima se não couber embaixo
    setUp(!!box && window.innerHeight - box.bottom < 260 && box.top > window.innerHeight - box.bottom);
    setActive(options.findIndex((o) => o.value === value));
    setOpen(true);
  };
  const pick = (o: SelectOption | undefined) => {
    if (!o || o.disabled) return;
    onChange(o.value);
    setOpen(false);
  };

  useEffect(() => {
    if (!open) return;
    const away = (e: PointerEvent) => !root.current?.contains(e.target as Node) && setOpen(false);
    document.addEventListener("pointerdown", away);
    return () => document.removeEventListener("pointerdown", away);
  }, [open]);

  useEffect(() => {
    if (open) list.current?.querySelector('[data-active="true"]')?.scrollIntoView({ block: "nearest" });
  }, [open, active]);

  const move = (delta: number) => {
    const enabled = options.map((o, i) => (o.disabled ? -1 : i)).filter((i) => i >= 0);
    if (!enabled.length) return;
    const at = enabled.indexOf(active);
    setActive(enabled[Math.min(enabled.length - 1, Math.max(0, at < 0 ? 0 : at + delta))]);
  };

  return (
    <div
      className="st-dropdown"
      ref={root}
      onKeyDown={(e) => {
        if (e.key === "Escape") setOpen(false);
        else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
          e.preventDefault();
          if (!open) show();
          else move(e.key === "ArrowDown" ? 1 : -1);
        } else if ((e.key === "Enter" || e.key === " ") && open) {
          e.preventDefault();
          pick(options[active]);
        }
      }}
    >
      <button className="st-dropdown-btn" aria-haspopup="listbox" aria-expanded={open} onClick={() => (open ? setOpen(false) : show())}>
        <span>{current?.label ?? value}</span>
        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
          <path d="m6 9 6 6 6-6" />
        </svg>
      </button>
      {open && (
        <ul className="st-dropdown-list" role="listbox" data-up={up} ref={list}>
          {options.map((o, i) => (
            <li
              key={o.value}
              role="option"
              aria-selected={o.value === value}
              aria-disabled={o.disabled}
              data-active={i === active}
              onPointerEnter={() => !o.disabled && setActive(i)}
              onClick={() => pick(o)}
            >
              {o.label}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
