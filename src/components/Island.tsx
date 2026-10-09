import { AnimatePresence, motion, type Transition } from "motion/react";
import { useEffect, useRef, useState, type CSSProperties } from "react";
import type { Activity } from "../activities/types";
import { native, type IslandShape, type Notch, type Platform, type Settings } from "../lib/native";

type Mode = "idle" | "compact" | "expanded";

/** Mola com leve overshoot: a forma "escorre" e assenta, em vez de só redimensionar. */
const LIQUID: Transition = { type: "spring", stiffness: 560, damping: 31, mass: 0.9 };
const CONTENT: Transition = { duration: 0.18, ease: [0.2, 0.8, 0.2, 1] };

const HIDDEN = (notch: Notch): IslandShape => ({ width: notch.width * 0.5, height: 0, radius: 0, ear: 0 });

function shapeFor(mode: Mode, notch: Notch, activity: Activity | null, hideIdle: boolean): IslandShape {
  switch (mode) {
    case "idle":
      return hideIdle ? HIDDEN(notch) : { width: notch.width, height: notch.height, radius: 10, ear: 6 };
    case "compact":
      return { width: notch.width + (notch.height + 14) * 2, height: notch.height, radius: 12, ear: 8 };
    case "expanded":
      return {
        width: activity!.expandedSize.width,
        height: notch.height + 6 + activity!.expandedSize.height,
        radius: 30,
        ear: 14,
      };
  }
}

type IslandProps = {
  notch: Notch;
  activity: Activity | null;
  hovered: boolean;
  settings: Settings;
  platform: Platform;
};

export function Island({ notch, activity, hovered, settings, platform }: IslandProps) {
  const [clicked, setClicked] = useState(false);
  useEffect(() => {
    if (!hovered) setClicked(false);
  }, [hovered]);

  const wantsOpen = settings.expandOn === "hover" ? hovered : hovered && clicked;
  const mode: Mode = !activity ? "idle" : wantsOpen ? "expanded" : activity.live ? "compact" : "idle";

  // Glass sem suporte nativo cai pra Translucent (ADR 0004)
  const style = settings.style === "glass" && !platform.glassSupported ? "translucent" : settings.style;
  const glassOpen = style === "glass" && mode === "expanded";
  const hideIdle = !notch.hasNotch && settings.hideIdleWithoutNotch;

  const shape = shapeFor(mode, notch, activity, hideIdle);
  // no Glass o preto recua pro Notch (ou some, sem Notch) e o vidro nativo assume a forma do Expanded
  const pill = glassOpen ? (notch.hasNotch ? shapeFor("idle", notch, activity, false) : HIDDEN(notch)) : shape;
  // de onde o vidro nasce e pra onde volta
  const rest = shapeFor(activity?.live ? "compact" : "idle", notch, activity, hideIdle);

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

  const pillContent =
    mode === "compact"
      ? activity!.compact({ side: notch.height + 14, height: notch.height })
      : mode === "expanded" && !glassOpen
        ? activity!.expanded({ topInset: notch.height + 6 })
        : null;
  const glassContent = glassOpen ? activity!.expanded({ topInset: notch.height + 6 }) : null;

  // Themed pinta Compact e Expanded; Idle continua preto (é só o Notch)
  const theme = style === "themed" && mode !== "idle" ? activity?.theme : null;
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
        onClick={() => settings.expandOn === "click" && activity && setClicked(true)}
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
                key={`${activity!.id}:${mode}`}
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
