import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState } from "react";
import { usePlatform, useSettings } from "../hooks/useSettings";
import { native } from "../lib/native";
import { Note, Range, Row, Section, Segmented, Toggle } from "./controls";
import icon from "../assets/icon.png";
import "./settings.css";

function useDisplays() {
  const [displays, setDisplays] = useState<string[]>([]);
  useEffect(() => {
    const load = () => native.listDisplays().then(setDisplays);
    load();
    // monitor conectado com a janela aberta
    window.addEventListener("focus", load);
    return () => window.removeEventListener("focus", load);
  }, []);
  return displays;
}

export function SettingsApp() {
  const { settings, update } = useSettings();
  const platform = usePlatform();
  const displays = useDisplays();
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [version, setVersion] = useState("");

  useEffect(() => {
    native.getAutostart().then(setAutostart);
    getVersion().then(setVersion);
  }, []);

  if (!settings || !platform) return null;

  const glassFallback = settings.style === "glass" && !platform.glassSupported;
  const missingDisplay = !["auto", "main", ...displays].includes(settings.display);

  return (
    <main className="settings">
      <header className="st-header">
        <img className="st-logo" src={icon} alt="" />
        <div>
          <h1>Dynamic Lite</h1>
          <small>Versão {version}</small>
        </div>
      </header>

      <Section title="Aparência">
        <Row label="Estilo" hint="Vale pro painel aberto; o compacto é sempre preto">
          <Segmented
            value={settings.style}
            onChange={(style) => update({ style })}
            options={[
              { value: "black", label: "Preto" },
              { value: "translucent", label: "Translúcido" },
              {
                value: "glass",
                label: "Vidro",
                disabled: !platform.glassSupported,
                title: platform.glassSupported ? "Liquid Glass" : "Requer macOS 26 ou mais novo",
              },
            ]}
          />
        </Row>
        {glassFallback && <Note>Liquid Glass requer macOS 26 ou mais novo. Usando Translúcido.</Note>}
        {(settings.style === "translucent" || glassFallback) && (
          <Row label="Opacidade">
            <Range
              value={Math.round(settings.opacity * 100)}
              min={60}
              max={100}
              step={5}
              format={(v) => `${v}%`}
              onChange={(v) => update({ opacity: v / 100 })}
            />
          </Row>
        )}
        <Row label="Cor de destaque" hint="Progresso e equalizer">
          <Segmented
            value={settings.accent}
            onChange={(accent) => update({ accent })}
            options={[
              { value: "artwork", label: "Da capa" },
              { value: "white", label: "Branca" },
            ]}
          />
        </Row>
      </Section>

      <Section title="Comportamento">
        <Row label="Abrir com">
          <Segmented
            value={settings.expandOn}
            onChange={(expandOn) => update({ expandOn })}
            options={[
              { value: "hover", label: "Passar o mouse" },
              { value: "click", label: "Clique" },
            ]}
          />
        </Row>
        {settings.expandOn === "hover" && (
          <Row label="Atraso pra abrir">
            <Range
              value={settings.hoverDelayMs}
              min={0}
              max={600}
              step={20}
              format={(v) => `${v} ms`}
              onChange={(hoverDelayMs) => update({ hoverDelayMs })}
            />
          </Row>
        )}
        <Row label="Toque no trackpad" hint="Um clique leve sentido no dedo quando a Island abre ou fecha">
          <Toggle checked={settings.haptics} onChange={(haptics) => update({ haptics })} />
        </Row>
      </Section>

      <Section title="Tela">
        <Row label="Mostrar em">
          <select className="st-select" value={settings.display} onChange={(e) => update({ display: e.target.value })}>
            <option value="auto">Automática (tela com notch)</option>
            <option value="main">Principal (com a menu bar)</option>
            {displays.length > 0 && <option disabled>──────────</option>}
            {displays.map((name) => (
              <option key={name} value={name}>{name}</option>
            ))}
            {missingDisplay && <option value={settings.display}>{settings.display} (desconectada)</option>}
          </select>
        </Row>
        <Row label="Esconder sem música" hint="Só em telas sem notch">
          <Toggle
            checked={settings.hideIdleWithoutNotch}
            onChange={(hideIdleWithoutNotch) => update({ hideIdleWithoutNotch })}
          />
        </Row>
      </Section>

      <Section title="Sistema">
        <Row label="Iniciar com o macOS">
          <Toggle
            checked={autostart ?? false}
            disabled={autostart === null}
            onChange={(enabled) => native.setAutostart(enabled).then(setAutostart)}
          />
        </Row>
        <Row label="Ícone na menu bar" hint="Sem ele, use o clique direito na Island">
          <Toggle checked={settings.showMenuBarIcon} onChange={(showMenuBarIcon) => update({ showMenuBarIcon })} />
        </Row>
      </Section>
    </main>
  );
}
