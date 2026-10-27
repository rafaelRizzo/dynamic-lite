import { useRef, useState, type PointerEvent } from "react";

type SliderProps = {
  /** 0-1 */
  value: number;
  color: string;
  /** Durante o arrasto (0-1). */
  onChange?: (ratio: number) => void;
  /** Ao soltar (0-1). */
  onCommit: (ratio: number) => void;
  onCancel?: () => void;
};

export function Slider({ value, color, onChange, onCommit, onCancel }: SliderProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [drag, setDrag] = useState<number | null>(null);
  const ratio = drag ?? value;

  const at = (e: PointerEvent) => {
    const r = ref.current!.getBoundingClientRect();
    return Math.min(1, Math.max(0, (e.clientX - r.left) / r.width));
  };
  const move = (e: PointerEvent) => {
    const r = at(e);
    setDrag(r);
    onChange?.(r);
  };

  return (
    <div
      ref={ref}
      className="slider"
      data-dragging={drag !== null}
      onPointerDown={(e) => {
        e.currentTarget.setPointerCapture(e.pointerId);
        move(e);
      }}
      onPointerMove={(e) => drag !== null && move(e)}
      onPointerUp={(e) => {
        if (drag === null) return;
        onCommit(at(e));
        setDrag(null);
      }}
      onPointerCancel={() => {
        setDrag(null);
        onCancel?.();
      }}
    >
      <div className="slider-fill" style={{ width: `${ratio * 100}%`, background: color }} />
    </div>
  );
}
