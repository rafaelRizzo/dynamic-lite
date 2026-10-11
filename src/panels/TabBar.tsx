import { motion } from "motion/react";
import type { Panel } from "../activities/types";

/** Altura da barra de abas, somada ao Expanded quando há mais de uma aba. */
export const TAB_BAR_HEIGHT = 40;

/** Cada aba ocupa um slot de largura fixa: nada muda de tamanho no clique, só a pílula anda. */
const SLOT = 34;
const GAP = 0;

type TabBarProps = { panels: Panel[]; active: string; onSelect: (id: string) => void };

export function TabBar({ panels, active, onSelect }: TabBarProps) {
  const index = panels.findIndex((p) => p.id === active);
  return (
    <div className="tab-bar" style={{ height: TAB_BAR_HEIGHT }} role="tablist">
      <div className="tab-track" style={{ width: panels.length * SLOT + (panels.length - 1) * GAP }}>
        {/* mola quase crítica: chega no slot sem passar do ponto */}
        <motion.span
          className="tab-pill"
          style={{ width: SLOT }}
          initial={false}
          animate={{ x: Math.max(0, index) * (SLOT + GAP), opacity: index < 0 ? 0 : 1 }}
          transition={{ type: "spring", stiffness: 900, damping: 56, mass: 1 }}
        />
        {panels.map((p) => (
          <button
            key={p.id}
            className="tab"
            style={{ width: SLOT }}
            role="tab"
            aria-selected={p.id === active}
            aria-label={p.label}
            data-tip={p.label}
            data-tip-side="bottom"
            data-active={p.id === active}
            onClick={() => onSelect(p.id)}
          >
            <span className="tab-icon">{p.icon}</span>
          </button>
        ))}
      </div>
    </div>
  );
}
