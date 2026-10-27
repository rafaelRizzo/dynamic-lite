import type { ReactNode } from "react";

export type Shortcut = { id: string; label: string; url: string; icon: ReactNode };

const Svg = ({ children }: { children: ReactNode }) => (
  <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
    {children}
  </svg>
);

const pane = (id: string) => `ms-settings:${id}`;

/** Páginas das Configurações do Windows 10/11 (`ms-settings:`). O usuário escolhe quais aparecem. */
export const SHORTCUTS: Shortcut[] = [
  { id: "wifi", label: "Wi-Fi", url: pane("network-wifi"), icon: <Svg><path d="M5 12.5a10 10 0 0 1 14 0M8 15.8a5.5 5.5 0 0 1 8 0M12 19.2h.01" /></Svg> },
  { id: "bluetooth", label: "Bluetooth", url: pane("bluetooth"), icon: <Svg><path d="m7 7 10 10-5 5V2l5 5L7 17" /></Svg> },
  { id: "sound", label: "Som", url: pane("sound"), icon: <Svg><path d="M11 5 6 9H3v6h3l5 4zM15.5 8.5a5 5 0 0 1 0 7M18.5 5.5a9 9 0 0 1 0 13" /></Svg> },
  { id: "battery", label: "Bateria", url: pane("batterysaver"), icon: <Svg><rect x="2" y="8" width="17" height="8" rx="2" /><path d="M22 11v2" /></Svg> },
  { id: "displays", label: "Telas", url: pane("display"), icon: <Svg><rect x="3" y="4" width="18" height="12" rx="2" /><path d="M8 20h8M12 16v4" /></Svg> },
  { id: "notifications", label: "Notificações", url: pane("notifications"), icon: <Svg><path d="M6 9a6 6 0 0 1 12 0c0 6 2 7 2 7H4s2-1 2-7M10 20a2 2 0 0 0 4 0" /></Svg> },
  { id: "focus", label: "Assistente de foco", url: pane("quiethours"), icon: <Svg><path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z" /></Svg> },
  { id: "network", label: "Rede", url: pane("network-status"), icon: <Svg><circle cx="12" cy="12" r="9" /><path d="M3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18" /></Svg> },
  { id: "keyboard", label: "Teclado", url: pane("easeofaccess-keyboard"), icon: <Svg><rect x="2" y="6" width="20" height="12" rx="2" /><path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M7 14h10" /></Svg> },
  { id: "trackpad", label: "Touchpad", url: pane("devices-touchpad"), icon: <Svg><rect x="4" y="3" width="16" height="18" rx="3" /><path d="M4 16h16" /></Svg> },
  { id: "wallpaper", label: "Papel de parede", url: pane("personalization-background"), icon: <Svg><rect x="3" y="4" width="18" height="16" rx="2" /><circle cx="9" cy="10" r="1.5" /><path d="m21 16-5-5-9 9" /></Svg> },
  { id: "appearance", label: "Aparência", url: pane("colors"), icon: <Svg><circle cx="12" cy="12" r="9" /><path d="M12 3a9 9 0 0 0 0 18z" /></Svg> },
  { id: "privacy", label: "Privacidade", url: pane("privacy"), icon: <Svg><rect x="5" y="11" width="14" height="9" rx="2" /><path d="M8 11V8a4 4 0 0 1 8 0v3" /></Svg> },
  { id: "general", label: "Sobre", url: pane("about"), icon: <Svg><path d="M4 7h10M18 7h2M4 17h2M10 17h10M14 5v4M6 15v4" /></Svg> },
];

export const SHORTCUT_COLUMNS = 4;
