import { motion } from "motion/react";
import { useEffect, useState } from "react";
import { useUpdate } from "../../hooks/useUpdate";
import type { UpdateInfo } from "../../lib/native";
import type { Activity } from "../types";

const EXPANDED = { width: 340, height: 104 };

const UpdateIcon = ({ size }: { size: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M12 17V7M7.5 11.5 12 7l4.5 4.5" />
  </svg>
);

function Compact({ info, side, height }: { info: UpdateInfo; side: number; height: number }) {
  const badge = Math.max(16, height - 14);
  return (
    <div className="up-compact" style={{ paddingInline: (side - badge) / 2 }}>
      <span className="up-badge" style={{ width: badge, height: badge }}>
        <UpdateIcon size={badge * 0.7} />
      </span>
      <span className="up-compact-version">{info.version}</span>
    </div>
  );
}

type ExpandedProps = {
  info: UpdateInfo;
  topInset: number;
  installing: boolean;
  error: string | null;
  onInstall: () => void;
  onLater: () => void;
};

function Expanded({ info, topInset, installing, error, onInstall, onLater }: ExpandedProps) {
  return (
    <div className="up-expanded" style={{ paddingTop: topInset }}>
      <div className="up-head">
        <span className="up-badge up-badge-large">
          <UpdateIcon size={24} />
        </span>
        <div className="up-meta">
          <div className="up-title">Atualização disponível</div>
          <div className="up-subtitle" data-error={!!error}>
            {error ? `Falhou: ${error}` : `Versão ${info.version} · você tem ${info.currentVersion}`}
          </div>
        </div>
      </div>
      <div className="up-actions">
        <motion.button className="up-btn" onClick={onLater} disabled={installing} whileTap={{ scale: 0.94 }}>
          Depois
        </motion.button>
        <motion.button className="up-btn up-btn-primary" onClick={onInstall} disabled={installing} whileTap={{ scale: 0.94 }}>
          {installing ? "Instalando…" : "Instalar e reabrir"}
        </motion.button>
      </div>
    </div>
  );
}

/** Atualização como Activity: aparece quando não há música tocando (Now Playing tem prioridade). */
export function useUpdateActivity(): Activity | null {
  const { info, installing, error, install } = useUpdate();
  // "Depois" esconde até a próxima checagem: cada checagem traz um objeto novo e reexibe
  const [dismissed, setDismissed] = useState<UpdateInfo | null>(null);
  useEffect(() => setDismissed(null), [info]);

  if (!info || dismissed === info) return null;
  return {
    id: "update",
    live: true,
    compact: ({ side, height }) => <Compact info={info} side={side} height={height} />,
    expanded: ({ topInset }) => (
      <Expanded
        info={info}
        topInset={topInset}
        installing={installing}
        error={error}
        onInstall={install}
        onLater={() => setDismissed(info)}
      />
    ),
    expandedSize: EXPANDED,
  };
}
