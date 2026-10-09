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

export const SpotifyIcon = ({ size = 18 }: IconProps) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor">
    <path d="M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20Zm4.6 14.4a.62.62 0 0 1-.86.2c-2.35-1.43-5.3-1.76-8.79-.96a.62.62 0 1 1-.28-1.21c3.81-.87 7.09-.5 9.73 1.12.3.18.39.56.2.85Zm1.22-2.73a.78.78 0 0 1-1.07.26c-2.69-1.65-6.78-2.13-9.96-1.17a.78.78 0 1 1-.45-1.49c3.63-1.1 8.15-.57 11.23 1.33.37.22.48.7.25 1.07Zm.1-2.84C14.7 8.92 9.37 8.74 6.3 9.68a.94.94 0 1 1-.54-1.8c3.53-1.07 9.4-.86 13.1 1.33a.94.94 0 0 1-.96 1.62Z" />
  </svg>
);
