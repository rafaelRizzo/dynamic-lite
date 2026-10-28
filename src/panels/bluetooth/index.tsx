import { useEffect, useState } from "react";
import type { Panel } from "../../activities/types";
import { native, type BluetoothBattery, type Settings } from "../../lib/native";
import { BluetoothIcon } from "../icons";

const WIDTH = 400;
const HEIGHT = 150;
const REFRESH_MS = 10_000;
const RING = 56;
const STROKE = 5;
/** Abaixo disso o anel fica vermelho; abaixo de `WARN`, âmbar. */
const LOW = 20;
const WARN = 40;

const levelOf = (pct: number) => (pct < LOW ? "low" : pct < WARN ? "warn" : "ok");

function Ring({ device }: { device: BluetoothBattery }) {
  const r = (RING - STROKE) / 2;
  const c = 2 * Math.PI * r;
  const pct = device.battery;
  return (
    <svg className="bt-ring" width={RING} height={RING} data-level={pct == null ? "none" : levelOf(pct)}>
      <circle cx={RING / 2} cy={RING / 2} r={r} fill="none" stroke="var(--track)" strokeWidth={STROKE} />
      {pct != null && (
        <circle
          cx={RING / 2}
          cy={RING / 2}
          r={r}
          fill="none"
          stroke="currentColor"
          strokeWidth={STROKE}
          strokeLinecap="round"
          className="bt-ring-arc"
          strokeDasharray={c}
          strokeDashoffset={c * (1 - pct / 100)}
          transform={`rotate(-90 ${RING / 2} ${RING / 2})`}
        />
      )}
      <text x="50%" y="50%" textAnchor="middle" dominantBaseline="central" className="bt-pct">
        {pct == null ? "--" : `${pct}%`}
      </text>
    </svg>
  );
}

/** Último resultado: ao voltar pra aba mostra na hora e atualiza por baixo. */
let last: BluetoothBattery[] | null = null;

function Expanded({ topInset }: { topInset: number }) {
  const [devices, setDevices] = useState<BluetoothBattery[] | null>(last);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    const load = () =>
      native
        .bluetoothBatteries()
        .then((d) => {
          last = d;
          setDevices(d);
          setFailed(false);
        })
        .catch((e) => {
          console.error("bluetooth_batteries falhou:", e);
          setFailed(true);
        });
    load();
    const id = setInterval(load, REFRESH_MS);
    return () => clearInterval(id);
  }, []);

  // só conectados; sem informar bateria o anel fica "--"
  const shown = (devices ?? []).filter((d) => d.connected).slice(0, 4);

  return (
    <div className="bt-expanded" style={{ paddingTop: topInset }}>
      {!devices ? (
        <div className="bt-empty">{failed ? "Não consegui ler o Bluetooth" : "Procurando dispositivos…"}</div>
      ) : shown.length === 0 ? (
        <div className="bt-empty">Nenhum dispositivo Bluetooth conectado</div>
      ) : (
        <div className="bt-cols">
          {shown.map((d) => (
            <div key={d.id} className="bt-device" data-connected={d.connected}>
              <Ring device={d} />
              <div className="bt-name" data-tip={d.name}>{d.name}</div>
              <div className="bt-sub">{d.battery != null && d.battery < LOW ? "bateria baixa" : d.connected ? "conectado" : "desconectado"}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

/** Bateria dos dispositivos Bluetooth (fones, mouse, teclado); só lê com a aba aberta. */
export function useBluetoothPanel(settings: Settings): Panel | null {
  if (!settings.showBluetooth) return null;
  return {
    id: "bluetooth",
    label: "Bateria Bluetooth",
    icon: <BluetoothIcon />,
    expanded: ({ topInset }) => <Expanded topInset={topInset} />,
    expandedSize: { width: WIDTH, height: HEIGHT },
  };
}
