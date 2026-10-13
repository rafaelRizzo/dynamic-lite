import { AnimatePresence, motion, type Transition, type Variants } from "motion/react";
import { useEffect, useRef, useState, type CSSProperties } from "react";
import type { Activity, Panel, Slots } from "../activities/types";
import { TAB_BAR_HEIGHT, TabBar } from "../panels/TabBar";
import { native, subscribe, type IslandShape, type Notch, type Platform, type Settings } from "../lib/native";

type Mode = "idle" | "compact" | "expanded";

/** Mola com leve overshoot: a forma "escorre" e assenta, em vez de só redimensionar. */
const LIQUID: Transition = { type: "spring", stiffness: 560, damping: 31, mass: 0.9 };
const CONTENT: Transition = { duration: 0.18, ease: [0.2, 0.8, 0.2, 1] };

/** Troca de aba: o conteúdo novo entra pelo lado pra onde a aba está, o antigo sai pro outro. */
const SLIDE = 26;
const PANEL: Variants = {
  enter: (dir: number) => ({ opacity: 0, x: dir * SLIDE, filter: "blur(4px)" }),
  center: { opacity: 1, x: 0, filter: "blur(0px)", transition: { type: "spring", stiffness: 720, damping: 46 } },
  exit: (dir: number) => ({ opacity: 0, x: -dir * SLIDE, filter: "blur(4px)", transition: { duration: 0.07 } }),
};

const TAB_KEY = "island.lastTab";

function loadTab(): string | null {
  try {
    return localStorage.getItem(TAB_KEY);
  } catch {
    return null;
  }
}

function saveTab(id: string | null) {
  try {
    if (id) localStorage.setItem(TAB_KEY, id);
    else localStorage.removeItem(TAB_KEY);
  } catch {
    // sem storage: só não lembra
  }
}

const HIDDEN = (notch: Notch): IslandShape => ({ width: notch.width * 0.5, height: 0, radius: 0, ear: 0 });

type Size = { width: number; height: number };

/** Largura mínima do Expanded: cabe a barra de abas mesmo com painéis estreitos. */
const MIN_EXPANDED_WIDTH = 300;

function shapeFor(mode: Mode, notch: Notch, expanded: Size | null, hideIdle: boolean): IslandShape {
  switch (mode) {
    case "idle":
      return hideIdle ? HIDDEN(notch) : { width: notch.width, height: notch.height, radius: 10, ear: 6 };
    case "compact":
      return { width: notch.width + (notch.height + 14) * 2, height: notch.height, radius: 12, ear: 8 };
    case "expanded":
      return {
        width: expanded!.width,
        height: notch.height + 6 + expanded!.height,
        radius: 30,
        ear: 14,
      };
  }
}

/** Cada lado do Notch fica com a primeira Activity (por prioridade) que o preenche. */
function composeSlots(activities: Activity[], layout: { side: number; height: number }): Slots {
  const slots = activities.map((a) => a.compact(layout));
  return { left: slots.find((s) => s.left)?.left, right: slots.find((s) => s.right)?.right };
}

type IslandProps = {
  notch: Notch;
  activities: Activity[];
  panels: Panel[];
  /** Painel que abre a Island sozinho, sem hover e fora da barra de abas (ex.: Alert). */
  forced: Panel | null;
  hovered: boolean;
  settings: Settings;
  platform: Platform;
};

export function Island({ notch, activities, panels, forced, hovered, settings, platform }: IslandProps) {
  const [clicked, setClicked] = useState(false);
  const [tab, setTab] = useState<string | null>(() => (settings.rememberTab ? loadTab() : null));
  // digitando num campo (tempo do Pomodoro, lembrete) a Island não fecha, mesmo com o mouse fora
  const [typing, setTyping] = useState(false);
  useEffect(() => {
    const isField = (t: EventTarget | null) => t instanceof HTMLInputElement || t instanceof HTMLTextAreaElement;
    const onIn = (e: FocusEvent) => setTyping(isField(e.target));
    const onOut = () => setTyping(false);
    document.addEventListener("focusin", onIn);
    document.addEventListener("focusout", onOut);
    return () => {
      document.removeEventListener("focusin", onIn);
      document.removeEventListener("focusout", onOut);
    };
  }, []);

  const present = hovered || typing;
  useEffect(() => {
    if (!present) {
      setClicked(false);
      // na próxima abertura volta pra primeira aba, a menos que lembre a última
      if (!settings.rememberTab) setTab(null);
    }
  }, [present, settings.rememberTab]);

  // desligar a opção apaga o que estava salvo
  useEffect(() => {
    saveTab(settings.rememberTab ? tab : null);
  }, [tab, settings.rememberTab]);

  const tabs = panels.filter((p) => !p.hidden);
  const panel = forced ?? tabs.find((p) => p.id === tab) ?? tabs[0] ?? null;
  const showTabs = !forced && tabs.length > 1;
  // pra que lado a aba nova está em relação à anterior (aviso forçado não conta)
  const index = forced ? -1 : tabs.findIndex((p) => p.id === panel?.id);
  const lastIndex = useRef(index);
  const direction = useRef(1);
  if (index >= 0 && index !== lastIndex.current) {
    direction.current = index > lastIndex.current ? 1 : -1;
    lastIndex.current = index;
  }
  // com abas, todas têm o tamanho da maior: se a Island encolhesse ao trocar de aba, a barra pularia pra cima
  // e o mouse ficaria fora da Hit Region (a Island fecharia no meio do clique)
  const sizing = showTabs ? tabs : panel ? [panel] : [];
  const expandedSize: Size | null = panel && {
    width: Math.max(MIN_EXPANDED_WIDTH, ...sizing.map((p) => p.expandedSize.width)),
    height: Math.max(...sizing.map((p) => p.expandedSize.height)) + (showTabs ? TAB_BAR_HEIGHT : 0),
  };

  const compactLayout = { side: notch.height + 14, height: notch.height };
  const slots = composeSlots(activities, compactLayout);
  const hasCompact = !!(slots.left || slots.right);

  const wantsOpen = !!forced || typing || (settings.expandOn === "hover" ? hovered : hovered && clicked);
  const mode: Mode = !panel ? (hasCompact ? "compact" : "idle") : wantsOpen ? "expanded" : hasCompact ? "compact" : "idle";

  // 3 dedos pro lado com a Island aberta: próxima/anterior aba, sem dar a volta
  const swipeState = useRef({ expanded: false, tabs, panel, showTabs, haptics: settings.haptics });
  swipeState.current = { expanded: mode === "expanded", tabs, panel, showTabs, haptics: settings.haptics };
  useEffect(
    () =>
      subscribe(
        native.onSwipe((dir) => {
          const { expanded, tabs, panel, showTabs, haptics } = swipeState.current;
          if (!expanded || !showTabs || !panel) return;
          const next = tabs[tabs.findIndex((p) => p.id === panel.id) + dir];
          if (!next) return;
          setTab(next.id);
          if (haptics) native.haptic();
        }),
      ),
    [],
  );

  // com a Island aberta o gesto de 3 dedos é só dela: o sistema não troca de mesa junto
  const captureSwipe = mode === "expanded" && showTabs && settings.swipeTabs;
  useEffect(() => {
    native.setSwipeCapture(captureSwipe);
  }, [captureSwipe]);

  // Glass sem suporte nativo cai pra Translucent (ADR 0004)
  const style = settings.style === "glass" && !platform.glassSupported ? "translucent" : settings.style;
  const glassOpen = style === "glass" && mode === "expanded";
  const hideIdle = !notch.hasNotch && settings.hideIdleWithoutNotch;

  const shape = shapeFor(mode, notch, expandedSize, hideIdle);
  // no Glass o preto recua pro Notch (ou some, sem Notch) e o vidro nativo assume a forma do Expanded
  const pill = glassOpen ? (notch.hasNotch ? shapeFor("idle", notch, expandedSize, false) : HIDDEN(notch)) : shape;
  // de onde o vidro nasce e pra onde volta
  const rest = shapeFor(hasCompact ? "compact" : "idle", notch, expandedSize, hideIdle);

  // Hit Region acompanha o alvo da animação, Ears incluídas
  const hitW = shape.width + shape.ear * 2;
  const hitH = shape.height;
  useEffect(() => {
    native.setIslandSize(hitW, hitH);
  }, [hitW, hitH]);

  const glassKey = JSON.stringify([glassOpen && shape, rest]);
  useEffect(() => {
    native.setGlass(glassOpen ? shape : null, rest);
  }, [glassKey]);

  const expanded = mode === "expanded";
  const wasExpanded = useRef(expanded);
  useEffect(() => {
    if (wasExpanded.current !== expanded && settings.haptics) native.haptic();
    wasExpanded.current = expanded;
  }, [expanded, settings.haptics]);

  const expandedContent = panel && (
    <div className="expanded-shell">
      <div className="expanded-panel" style={{ height: `calc(100% - ${showTabs ? TAB_BAR_HEIGHT : 0}px)` }}>
        <AnimatePresence initial={false} mode="popLayout" custom={direction.current}>
          <motion.div
            key={panel.id}
            className="expanded-panel-body"
            custom={direction.current}
            variants={PANEL}
            initial="enter"
            animate="center"
            exit="exit"
          >
            {panel.expanded({ topInset: notch.height + 6 })}
          </motion.div>
        </AnimatePresence>
      </div>
      {showTabs && <TabBar panels={tabs} active={panel.id} onSelect={setTab} />}
    </div>
  );
  const compactContent = (
    <div className="compact" style={{ gridTemplateColumns: `${compactLayout.side}px 1fr ${compactLayout.side}px` }}>
      <div className="compact-slot">{slots.left}</div>
      <span />
      <div className="compact-slot">{slots.right}</div>
    </div>
  );
  const pillContent = mode === "compact" ? compactContent : mode === "expanded" && !glassOpen ? expandedContent : null;
  const glassContent = glassOpen ? expandedContent : null;

  // Themed pinta Compact e Expanded; Idle continua preto (é só o Notch)
  const themeSource = mode === "expanded" ? panel?.theme : activities.find((a) => a.theme)?.theme;
  const theme = style === "themed" && mode !== "idle" ? themeSource : null;
  const tint = theme ? theme.background : `rgb(0 0 0 / ${style === "translucent" && expanded ? settings.opacity : 1})`;
  // no Themed a cor preenche tudo, faixa do Notch e Ears incluídas
  const islandVars = {
    "--island": theme ? theme.background : "#000",
    "--island-tint": tint,
    "--solid": `${notch.height}px`,
  } as CSSProperties;

  return (
    <div
      className="stage"
      onContextMenu={(e) => {
        e.preventDefault();
        native.showContextMenu();
      }}
    >
      <motion.div
        className="island-wrap"
        style={islandVars}
        initial={false}
        animate={{ width: pill.width, height: pill.height }}
        transition={LIQUID}
        onClick={() => settings.expandOn === "click" && panel && setClicked(true)}
      >
        <motion.span className="ear ear-left" initial={false} animate={{ width: pill.ear, height: pill.ear }} transition={LIQUID} />
        <motion.span className="ear ear-right" initial={false} animate={{ width: pill.ear, height: pill.ear }} transition={LIQUID} />
        <motion.div
          className="island"
          data-ink={theme?.ink ?? "light"}
          initial={false}
          animate={{ borderBottomLeftRadius: pill.radius, borderBottomRightRadius: pill.radius }}
          transition={LIQUID}
        >
          <AnimatePresence initial={false}>
            {pillContent && (
              <motion.div
                key={mode}
                className="island-content"
                style={{ width: pill.width, height: pill.height, left: `calc(50% - ${pill.width / 2}px)` }}
                initial={{ opacity: 0, filter: "blur(8px)", scale: 0.94 }}
                animate={{ opacity: 1, filter: "blur(0px)", scale: 1, transition: { ...CONTENT, delay: 0.03 } }}
                exit={{ opacity: 0, filter: "blur(8px)", scale: 0.94, transition: { duration: 0.1 } }}
              >
                {pillContent}
              </motion.div>
            )}
          </AnimatePresence>
        </motion.div>
      </motion.div>

      {/* conteúdo do Expanded no Glass: o vidro em si é nativo, atrás do webview */}
      <AnimatePresence>
        {glassContent && (
          <motion.div
            key="glass-content"
            className="glass-content"
            style={{ width: shape.width, height: shape.height, left: `calc(50% - ${shape.width / 2}px)` }}
            initial={{ opacity: 0, filter: "blur(8px)", scale: 0.94 }}
            animate={{ opacity: 1, filter: "blur(0px)", scale: 1, transition: { ...CONTENT, delay: 0.03 } }}
            exit={{ opacity: 0, filter: "blur(8px)", scale: 0.94, transition: { duration: 0.1 } }}
          >
            {glassContent}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
