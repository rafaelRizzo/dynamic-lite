import type { CSSProperties, ReactNode } from "react";

const Svg = ({ children, className }: { children: ReactNode; className?: string }) => (
  <svg className={className} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    {children}
  </svg>
);

export const AudioFileIcon = () => (
  <Svg>
    <path d="M9 18V5l11-2v13" />
    <circle cx="6" cy="18" r="3" />
    <circle cx="17" cy="16" r="3" />
  </Svg>
);
export const ImageFileIcon = () => (
  <Svg>
    <rect x="3" y="4" width="18" height="16" rx="3" />
    <circle cx="9" cy="10" r="1.6" />
    <path d="m21 16-5-5-9 9" />
  </Svg>
);
export const GenericFileIcon = () => (
  <Svg>
    <path d="M14 3H7.5A2.5 2.5 0 0 0 5 5.5v13A2.5 2.5 0 0 0 7.5 21h9a2.5 2.5 0 0 0 2.5-2.5V8Z" />
    <path d="M14 3v5h5" />
  </Svg>
);
export const OpenIcon = () => (
  <Svg>
    <path d="M14 4h6v6M20 4 10 14M18 14v4.5A1.5 1.5 0 0 1 16.5 20h-11A1.5 1.5 0 0 1 4 18.5v-11A1.5 1.5 0 0 1 5.5 6H10" />
  </Svg>
);
export const RevealIcon = () => (
  <Svg>
    <path d="M3 7.5A2.5 2.5 0 0 1 5.5 5H9l2 2.5h7.5A2.5 2.5 0 0 1 21 10v7.5a2.5 2.5 0 0 1-2.5 2.5h-13A2.5 2.5 0 0 1 3 17.5Z" />
  </Svg>
);
export const CloseIcon = () => (
  <Svg>
    <path d="M6 6l12 12M18 6 6 18" />
  </Svg>
);
export const AddFileIcon = () => (
  <Svg>
    <path d="M12 5v14M5 12h14" />
  </Svg>
);

export const PasteImageIcon = () => (
  <Svg>
    <rect x="5" y="4" width="14" height="17" rx="2.5" />
    <path d="M9 4V3h6v1M8.5 16.5l2.5-3 2 2 1.5-1.5 2 2.5" />
  </Svg>
);

/** Pasta com seta: a seta balança enquanto algo está sendo arrastado por cima. */
export const DropFolder = () => (
  <svg className="fx-folder" viewBox="0 0 48 48" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
    <path d="M6 14a5 5 0 0 1 5-5h8l4 5h14a5 5 0 0 1 5 5v14a5 5 0 0 1-5 5H11a5 5 0 0 1-5-5Z" />
    <g className="fx-arrow">
      <path d="M24 33V21M18.5 26.5 24 21l5.5 5.5" />
    </g>
  </svg>
);

/** Quando a conversão termina: o anel se desenha, o visto aparece com um salto e faíscas saem em volta. */
export function DoneCheck() {
  return (
    <svg className="fx-done" viewBox="0 0 52 52" aria-hidden="true">
      {Array.from({ length: 8 }, (_, i) => (
        <line key={i} className="fx-spark" x1="26" y1="-9" x2="26" y2="-3" style={{ "--a": `${i * 45}deg` } as CSSProperties} />
      ))}
      <circle className="fx-ring" cx="26" cy="26" r="22" />
      <g className="fx-pop">
        <path className="fx-tick" d="M15.5 27.5l7.5 7.5 14-15.5" />
      </g>
    </svg>
  );
}

/** Barrinhas que dançam enquanto converte. */
export function Wave() {
  return (
    <span className="fx-wave" aria-hidden="true">
      <i />
      <i />
      <i />
      <i />
      <i />
    </span>
  );
}
