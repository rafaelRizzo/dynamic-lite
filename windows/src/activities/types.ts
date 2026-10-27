import type { ReactNode } from "react";
import type { Theme } from "../hooks/useArtworkColor";

/** Os dois lados do Notch no Compact. */
export type Slots = { left?: ReactNode; right?: ReactNode };

/** Algo em andamento que aparece no Compact. Existir já significa "vivo". */
export type Activity = {
  id: string;
  /** Recebe a largura que cada lado do Compact tem fora do Notch e a altura do Notch. */
  compact: (layout: { side: number; height: number }) => Slots;
  /** Cor da capa no Style Themed; sem ela, fica preto. */
  theme?: Theme | null;
};

/** Uma aba do Expanded. */
export type Panel = {
  id: string;
  label: string;
  icon: ReactNode;
  /** `topInset`: espaço livre no topo (Notch no Style Black/Translucent, respiro no Glass Card). */
  expanded: (layout: { topInset: number }) => ReactNode;
  /** Tamanho do conteúdo sem o `topInset` e sem a barra de abas. */
  expandedSize: { width: number; height: number };
  /** Fundo do Expanded no Style Themed; sem ele, fica preto. */
  theme?: Theme | null;
  /** Fora da barra de abas: só aparece quando forçado (ex.: Alert). */
  hidden?: boolean;
};
