import type { ReactNode } from "react";
import type { Theme } from "../hooks/useArtworkColor";

/** Algo em andamento que a Island pode mostrar. */
export type Activity = {
  id: string;
  /** Aparece em Compact sem hover. Se false, só aparece no Expanded. */
  live: boolean;
  /** Recebe a largura que cada lado do Compact tem fora do Notch e a altura do Notch. */
  compact: (layout: { side: number; height: number }) => ReactNode;
  /** `topInset`: espaço livre no topo (Notch no Style Black/Translucent, respiro no Glass Card). */
  expanded: (layout: { topInset: number }) => ReactNode;
  /** Tamanho do conteúdo do Expanded sem o `topInset`. */
  expandedSize: { width: number; height: number };
  /** Fundo do Expanded no Style Themed; sem ele, fica preto. */
  theme?: Theme | null;
};
