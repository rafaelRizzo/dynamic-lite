import type { ReactNode } from "react";

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
