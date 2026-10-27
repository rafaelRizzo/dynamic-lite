type IconProps = { size?: number };

export const PlayIcon = ({ size = 22 }: IconProps) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor">
    <path d="M7 4.8v14.4c0 .9 1 1.4 1.7.9l10.6-7.2a1.1 1.1 0 0 0 0-1.8L8.7 3.9C8 3.4 7 3.9 7 4.8Z" />
  </svg>
);

export const PauseIcon = ({ size = 22 }: IconProps) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor">
    <rect x="5.5" y="4" width="4.5" height="16" rx="1.4" />
    <rect x="14" y="4" width="4.5" height="16" rx="1.4" />
  </svg>
);

export const NextIcon = ({ size = 20 }: IconProps) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor">
    <path d="M3 6.2v11.6c0 .8.9 1.3 1.6.8l8.4-5.8a1 1 0 0 0 0-1.6L4.6 5.4C3.9 4.9 3 5.4 3 6.2Zm9 0v11.6c0 .8.9 1.3 1.6.8l8.4-5.8a1 1 0 0 0 0-1.6l-8.4-5.8c-.7-.5-1.6 0-1.6.8Z" />
  </svg>
);

export const PrevIcon = ({ size = 20 }: IconProps) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" style={{ transform: "scaleX(-1)" }}>
    <path d="M3 6.2v11.6c0 .8.9 1.3 1.6.8l8.4-5.8a1 1 0 0 0 0-1.6L4.6 5.4C3.9 4.9 3 5.4 3 6.2Zm9 0v11.6c0 .8.9 1.3 1.6.8l8.4-5.8a1 1 0 0 0 0-1.6l-8.4-5.8c-.7-.5-1.6 0-1.6.8Z" />
  </svg>
);

export const VolumeLowIcon = ({ size = 16 }: IconProps) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor">
    <path d="M4 9.5v5c0 .6.4 1 1 1h3l4.3 3.6c.7.5 1.7.1 1.7-.8V5.7c0-.9-1-1.3-1.7-.8L8 8.5H5c-.6 0-1 .4-1 1Z" />
    <path d="M16.5 9a4 4 0 0 1 0 6" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" fill="none" />
  </svg>
);

export const VolumeHighIcon = ({ size = 16 }: IconProps) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor">
    <path d="M3 9.5v5c0 .6.4 1 1 1h3l4.3 3.6c.7.5 1.7.1 1.7-.8V5.7c0-.9-1-1.3-1.7-.8L7 8.5H4c-.6 0-1 .4-1 1Z" />
    <path d="M15.5 9a4 4 0 0 1 0 6M18.5 6.5a7.5 7.5 0 0 1 0 11" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" fill="none" />
  </svg>
);

export const VolumeMutedIcon = ({ size = 16 }: IconProps) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor">
    <path d="M3 9.5v5c0 .6.4 1 1 1h3l4.3 3.6c.7.5 1.7.1 1.7-.8V5.7c0-.9-1-1.3-1.7-.8L7 8.5H4c-.6 0-1 .4-1 1Z" />
    <path d="m16 9.5 5 5m0-5-5 5" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" fill="none" />
  </svg>
);

export const ShuffleIcon = ({ size = 18 }: IconProps) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M3 7h3.5c2 0 3 1 4.2 2.8l2.6 4.4C14.5 16 15.5 17 17.5 17H21" />
    <path d="M3 17h3.5c1.4 0 2.3-.5 3.1-1.4M14.4 8.4c.8-.9 1.7-1.4 3.1-1.4H21" />
    <path d="m18 4 3 3-3 3M18 14l3 3-3 3" />
  </svg>
);

export const RepeatIcon = ({ size = 18 }: IconProps) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M17 3l3 3-3 3" />
    <path d="M4 12V9.5A3.5 3.5 0 0 1 7.5 6H20M7 21l-3-3 3-3" />
    <path d="M20 12v2.5a3.5 3.5 0 0 1-3.5 3.5H4" />
  </svg>
);

export const RepeatOneIcon = ({ size = 18 }: IconProps) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
    <path d="M17 3l3 3-3 3" />
    <path d="M4 12V9.5A3.5 3.5 0 0 1 7.5 6H20M7 21l-3-3 3-3" />
    <path d="M20 12v2.5a3.5 3.5 0 0 1-3.5 3.5H4" />
    <path d="M11 10.5l1.5-1v5" strokeWidth="1.7" />
  </svg>
);
