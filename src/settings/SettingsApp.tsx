import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState } from "react";
import { usePlatform, useSettings } from "../hooks/useSettings";
import { useUpdate } from "../hooks/useUpdate";
import { native } from "../lib/native";
import { SHORTCUTS } from "../panels/shortcuts/catalog";
import { Chips, DurationField, Note, Range, Row, Section, Segmented, Toggle } from "./controls";
import icon from "../assets/icon.png";
import "./settings.css";

function useSounds() {
  const [sounds, setSounds] = useState<string[]>([]);
  useEffect(() => {
    native.listSounds().then(setSounds);
  }, []);
  return sounds;
}

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
  const sounds = useSounds();
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
        <Row label="Estilo" hint="Vale pro painel aberto; o compacto é preto, exceto no Capa">
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
              { value: "themed", label: "Capa", title: "Fundo na cor da capa da música" },
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
        {settings.style === "themed" ? (
          <Note>Compacta e aberta seguem a cor da capa; texto e controles ficam claros ou escuros pra manter o contraste.</Note>
        ) : (
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
        )}
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
        <Row label="Gesto de 3 dedos" hint="Com a Island aberta e o mouse nela, deslize 3 dedos pro lado pra trocar de aba">
          <Toggle checked={settings.swipeTabs} onChange={(swipeTabs) => update({ swipeTabs })} />
        </Row>
        {settings.swipeTabs && (
          <Row label="Inverter a direção do gesto">
            <Toggle checked={settings.invertSwipe} onChange={(invertSwipe) => update({ invertSwipe })} />
          </Row>
        )}
        <Row label="Atraso pra fechar" hint="Quanto a Island espera depois que o mouse sai. Voltar nesse tempo cancela">
          <Range
            value={settings.hoverExitDelayMs}
            min={0}
            max={2000}
            step={50}
            format={(v) => `${v} ms`}
            onChange={(hoverExitDelayMs) => update({ hoverExitDelayMs })}
          />
        </Row>
        <Row label="Toque no trackpad" hint="Um clique leve sentido no dedo quando a Island abre ou fecha">
          <Toggle checked={settings.haptics} onChange={(haptics) => update({ haptics })} />
        </Row>
      </Section>

      <Section title="Claude Code">
        <Row label="Aba de uso do Claude Code" hint="Custo de hoje, sessão de 5h e limite semanal do plano. Desligada, nada é lido">
          <Toggle checked={settings.showClaudeUsage} onChange={(showClaudeUsage) => update({ showClaudeUsage })} />
        </Row>
        {settings.showClaudeUsage && (
          <>
            <Row
              label="Consultar o limite na conta"
              hint="Usa o token do Claude Code no Keychain e pergunta à Anthropic (api.anthropic.com), como o /usage. O token só fica na memória"
            >
              <Toggle checked={settings.claudeLiveLimits} onChange={(claudeLiveLimits) => update({ claudeLiveLimits })} />
            </Row>
            <Note>
              {settings.claudeLiveLimits
                ? "Se a consulta falhar, o app lê o arquivo ~/.claude/dynano-usage.json do statusline."
                : "Sem a consulta, o limite vem do statusline: grave o JSON recebido em ~/.claude/dynano-usage.json (cat > ~/.claude/dynano-usage.json). Só atualiza com o Claude Code no terminal."}
            </Note>
          </>
        )}
      </Section>

      <Section title="Agenda">
        <Row label="Aba da agenda" hint="Eventos do app Calendário e lembretes com hora">
          <Toggle checked={settings.showAgenda} onChange={(showAgenda) => update({ showAgenda })} />
        </Row>
        {settings.showAgenda && (
          <Row label="Avisar antes de um evento" hint="A Island abre sozinha e toca o som">
            <Segmented
              value={String(settings.calendarLeadMin)}
              onChange={(v) => update({ calendarLeadMin: Number(v) })}
              options={[
                { value: "0", label: "Nunca" },
                { value: "5", label: "5 min" },
                { value: "10", label: "10 min" },
                { value: "15", label: "15 min" },
                { value: "30", label: "30 min" },
              ]}
            />
          </Row>
        )}
      </Section>

      <Section title="Pomodoro">
        <Row label="Aba do Pomodoro" hint="Com o timer rodando, o tempo aparece na Island fechada">
          <Toggle checked={settings.showPomodoro} onChange={(showPomodoro) => update({ showPomodoro })} />
        </Row>
        <Row label="Foco" hint="De 1 minuto a 24 horas: 25, 90m, 1h30">
          <DurationField value={settings.focusMin} onChange={(focusMin) => update({ focusMin })} />
        </Row>
        <Row label="Pausa curta">
          <DurationField value={settings.shortBreakMin} onChange={(shortBreakMin) => update({ shortBreakMin })} />
        </Row>
        <Row label="Pausa longa">
          <DurationField value={settings.longBreakMin} onChange={(longBreakMin) => update({ longBreakMin })} />
        </Row>
        <Row label="Pausa longa a cada">
          <Range value={settings.longEvery} min={2} max={8} step={1} format={(v) => `${v} focos`} onChange={(longEvery) => update({ longEvery })} />
        </Row>
        <Row label="Iniciar a próxima fase sozinho" hint="Sem isso, o aviso oferece iniciar">
          <Toggle checked={settings.autoStartNext} onChange={(autoStartNext) => update({ autoStartNext })} />
        </Row>
        <Row label="Som ao terminar" hint="Também toca nos lembretes">
          <div className="st-inline">
            <select className="st-select" value={settings.sfx} onChange={(e) => update({ sfx: e.target.value })}>
              {!sounds.includes(settings.sfx) && <option value={settings.sfx}>{settings.sfx}</option>}
              {sounds.map((name) => (
                <option key={name} value={name}>{name}</option>
              ))}
            </select>
            <button className="st-button" onClick={() => native.playSound(settings.sfx, settings.sfxVolume)}>Testar</button>
          </div>
        </Row>
        <Row label="Volume do som">
          <Range value={Math.round(settings.sfxVolume * 100)} min={10} max={100} step={5} format={(v) => `${v}%`} onChange={(v) => update({ sfxVolume: v / 100 })} />
        </Row>
      </Section>

      <Section title="Atalhos">
        <Row label="Aba de atalhos" hint="Abre painéis dos Ajustes do Sistema">
          <Toggle checked={settings.showShortcuts} onChange={(showShortcuts) => update({ showShortcuts })} />
        </Row>
        {settings.showShortcuts && (
          <Chips
            options={SHORTCUTS.map((s) => ({ value: s.id, label: s.label }))}
            selected={settings.shortcuts}
            max={12}
            onChange={(shortcuts) => update({ shortcuts })}
          />
        )}
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
