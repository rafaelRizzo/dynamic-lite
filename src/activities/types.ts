import type { ReactNode } from "react";

/** Algo em andamento que a Island pode mostrar. */
export type Activity = {
  id: string;
  /** Aparece em Compact sem hover. Se false, só aparece ao passar o mouse (Expanded). */
  live: boolean;
  compact: ReactNode;
  expanded: ReactNode;
  /** Tamanho da Island Expanded (sem as Ears). */
  expandedSize: { width: number; height: number };
};
