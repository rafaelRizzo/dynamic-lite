import { motion } from "motion/react";
import type { Panel } from "../../activities/types";
import { native, type Settings } from "../../lib/native";
import { GridIcon } from "../icons";
import { SHORTCUTS, SHORTCUT_COLUMNS } from "./catalog";

const WIDTH = 400;
const TILE_HEIGHT = 58;
const GAP = 8;
/** respiro inferior */
const BOTTOM = 14;

function Expanded({ ids, topInset }: { ids: string[]; topInset: number }) {
  const items = ids.map((id) => SHORTCUTS.find((s) => s.id === id)).filter((s) => !!s);
  return (
    <div className="sc-expanded" style={{ paddingTop: topInset }}>
      {items.length === 0 ? (
        <div className="sc-empty">Escolha os atalhos em Ajustes</div>
      ) : (
        <div className="sc-grid" style={{ gridTemplateColumns: `repeat(${SHORTCUT_COLUMNS}, 1fr)`, gridAutoRows: TILE_HEIGHT, gap: GAP }}>
          {items.map((s) => (
            <motion.button
              key={s.id}
              className="sc-tile"
              onClick={() => native.openSystemSettings(s.url).catch((e) => console.error("open_system_settings falhou:", e))}
              whileHover={{ scale: 1.04 }}
              whileTap={{ scale: 0.92 }}
              transition={{ type: "spring", stiffness: 600, damping: 22 }}
            >
              {s.icon}
              <span>{s.label}</span>
            </motion.button>
          ))}
        </div>
      )}
    </div>
  );
}

/** Atalhos pros painéis dos Ajustes do Sistema, escolhidos nos Ajustes do app. */
export function useShortcutsPanel(settings: Settings): Panel | null {
  if (!settings.showShortcuts) return null;
  const rows = Math.max(1, Math.ceil(settings.shortcuts.length / SHORTCUT_COLUMNS));
  return {
    id: "shortcuts",
    label: "Atalhos",
    icon: <GridIcon />,
    expanded: ({ topInset }) => <Expanded ids={settings.shortcuts} topInset={topInset} />,
    expandedSize: { width: WIDTH, height: rows * TILE_HEIGHT + (rows - 1) * GAP + BOTTOM },
  };
}
