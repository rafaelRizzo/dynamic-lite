import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState } from "react";
import { usePlatform, useSettings } from "../hooks/useSettings";
import { useUpdate } from "../hooks/useUpdate";
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

type UpdatesSectionProps = {
  update: ReturnType<typeof useUpdate>;
  autoUpdate: boolean;
  onAutoUpdate: (v: boolean) => void;
};

function UpdatesSection({ update, autoUpdate, onAutoUpdate }: UpdatesSectionProps) {
  const { info, setInfo, installing, error, install } = update;
  const [checking, setChecking] = useState(false);
  const [checked, setChecked] = useState(false);
  const [checkError, setCheckError] = useState<string | null>(null);

  const check = () => {
    setChecking(true);
    setCheckError(null);
    native
      .checkUpdate()
      .then((found) => {
        setInfo(found);
        setChecked(true);
      })
      .catch((e) => setCheckError(String(e)))
      .finally(() => setChecking(false));
  };

  const status = error
    ? `Falhou: ${error}`
    : checkError
      ? `Não deu pra verificar: ${checkError}`
      : installing
        ? "Instalando, o app vai reabrir…"
        : info
          ? `Versão ${info.version} disponível (você tem ${info.currentVersion})`
          : checked
            ? "Você está na versão mais recente"
            : undefined;

  return (
    <Section title="Atualizações">
      <Row label="Verificar automaticamente" hint="Ao abrir o app e uma vez por dia">
        <Toggle checked={autoUpdate} onChange={onAutoUpdate} />
      </Row>
      <Row label={info ? "Atualização disponível" : "Procurar atualização"} hint={status}>
        {info ? (
          <button className="st-button st-button-primary" onClick={install} disabled={installing}>
            {installing ? "Instalando…" : "Instalar e reabrir"}
          </button>
        ) : (
          <button className="st-button" onClick={check} disabled={checking}>
            {checking ? "Verificando…" : "Verificar agora"}
          </button>
        )}
      </Row>
    </Section>
  );
}

export function SettingsApp() {
  const { settings, update } = useSettings();
  const platform = usePlatform();
  const displays = useDisplays();
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [version, setVersion] = useState("");
  const appUpdate = useUpdate();
  const autoUpdate = settings?.autoUpdate;

  useEffect(() => {
    native.getAutostart().then(setAutostart);
    getVersion().then(setVersion);
  }, []);

  // abrir os Ajustes já checa, sem esperar a checagem diária
  useEffect(() => {
    if (autoUpdate) native.checkUpdate().catch(() => {});
  }, [autoUpdate]);

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
        {appUpdate.info && (
          <button
            className="st-button st-button-primary st-header-update"
            onClick={appUpdate.install}
            disabled={appUpdate.installing}
            title={`Instala a versão ${appUpdate.info.version} e reabre o app`}
          >
            {appUpdate.installing ? "Instalando…" : `Atualizar pra ${appUpdate.info.version}`}
          </button>
        )}
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
        <Row label="Mesas" hint="Só a atual: fica presa na mesa ativa ao escolher">
          <Segmented
            value={settings.allSpaces ? "all" : "current"}
            onChange={(v) => update({ allSpaces: v === "all" })}
            options={[
              { value: "all", label: "Todas" },
              { value: "current", label: "Só a atual" },
            ]}
          />
        </Row>
        <Row label="Esconder no Mission Control" hint="Não cobre a barra de mesas no topo">
          <Toggle
            checked={settings.hideInMissionControl}
            onChange={(hideInMissionControl) => update({ hideInMissionControl })}
          />
        </Row>
        <Row label="Esconder sem música" hint="Só em telas sem notch">
          <Toggle
            checked={settings.hideIdleWithoutNotch}
            onChange={(hideIdleWithoutNotch) => update({ hideIdleWithoutNotch })}
          />
        </Row>
      </Section>

      <UpdatesSection update={appUpdate} autoUpdate={settings.autoUpdate} onAutoUpdate={(autoUpdate) => update({ autoUpdate })} />

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
