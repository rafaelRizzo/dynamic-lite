const BARS = [0.9, 0.62, 1.05, 0.75];

export function Equalizer({ playing, color, height = 14 }: { playing: boolean; color: string; height?: number }) {
  return (
    <div className="eq" data-playing={playing} style={{ height, color }}>
      {BARS.map((speed, i) => (
        <span key={i} style={{ animationDuration: `${speed}s`, animationDelay: `${-i * 0.23}s` }} />
      ))}
    </div>
  );
}
