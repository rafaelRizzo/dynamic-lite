import { AnimatePresence, motion, type Transition } from "motion/react";
import { useEffect } from "react";
import type { Activity } from "../activities/types";
import { native, type Notch } from "../lib/native";

type Mode = "idle" | "compact" | "expanded";

/** Mola com leve overshoot: a forma "escorre" e assenta, em vez de só redimensionar. */
const LIQUID: Transition = { type: "spring", stiffness: 380, damping: 26, mass: 0.9 };
const CONTENT: Transition = { duration: 0.22, ease: [0.2, 0.8, 0.2, 1] };

function shape(mode: Mode, notch: Notch, activity: Activity | null) {
  const compactSide = notch.height + 14;
  switch (mode) {
    case "idle":
      return { width: notch.width, height: notch.height, radius: 10, ear: 6 };
    case "compact":
      return { width: notch.width + compactSide * 2, height: notch.height, radius: 12, ear: 8 };
    case "expanded":
      return { ...activity!.expandedSize, radius: 30, ear: 14 };
  }
}

export function Island({ notch, activity, hovered }: { notch: Notch; activity: Activity | null; hovered: boolean }) {
  const mode: Mode = !activity ? "idle" : hovered ? "expanded" : activity.live ? "compact" : "idle";
  const { width, height, radius, ear } = shape(mode, notch, activity);

  // a Hit Region acompanha o alvo da animação, Ears incluídas
  useEffect(() => {
    native.setIslandSize(width + ear * 2, height);
  }, [width, height, ear]);

  const content = mode === "compact" ? activity!.compact : mode === "expanded" ? activity!.expanded : null;

  return (
    <div className="stage">
      <motion.div className="island-wrap" initial={false} animate={{ width, height }} transition={LIQUID}>
        <motion.span className="ear ear-left" initial={false} animate={{ width: ear, height: ear }} transition={LIQUID} />
        <motion.span className="ear ear-right" initial={false} animate={{ width: ear, height: ear }} transition={LIQUID} />
        <motion.div
          className="island"
          initial={false}
          animate={{ borderBottomLeftRadius: radius, borderBottomRightRadius: radius }}
          transition={LIQUID}
        >
          <AnimatePresence initial={false}>
            {content && (
              <motion.div
                key={`${activity!.id}:${mode}`}
                className="island-content"
                style={{ width, height, left: `calc(50% - ${width / 2}px)` }}
                initial={{ opacity: 0, filter: "blur(8px)", scale: 0.94 }}
                animate={{ opacity: 1, filter: "blur(0px)", scale: 1, transition: { ...CONTENT, delay: 0.06 } }}
                exit={{ opacity: 0, filter: "blur(8px)", scale: 0.94, transition: { duration: 0.12 } }}
              >
                {content}
              </motion.div>
            )}
          </AnimatePresence>
        </motion.div>
      </motion.div>
    </div>
  );
}
