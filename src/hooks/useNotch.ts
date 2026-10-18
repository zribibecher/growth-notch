import { useCallback, useEffect, useRef, useState } from "react";
import { callQuietly } from "../lib/native";
import type { NotchStage } from "../types/notch";
import { useNativeEvent } from "./useNativeEvent";

/** Must outlast the CSS spring (--notch-dur), or the native window would clip the retract. */
const WINDOW_SHRINK_DELAY_MS = 600;
/** Leaving and re-entering within this time keeps the pill open. */
const CLOSE_DELAY_MS = 45;

interface Options {
  /** True while the user is typing in the Setup field: stay open when the cursor leaves, and let the panel take keyboard focus. */
  typing: boolean;
  /** Called whenever the cursor arrives on the pill. */
  onHover: () => void;
}

/**
 * The pill's three stages and the native window that backs them. The cursor itself is watched by
 * the native side (WebKit sends no hover events to an unfocused window), which sends
 * `notch://hover`.
 */
export function useNotch({ typing, onHover }: Options) {
  const [stage, setStage] = useState<NotchStage>("idle");
  const stageRef = useRef<NotchStage>("idle");
  const typingRef = useRef(typing);
  const onHoverRef = useRef(onHover);
  const closeTimer = useRef<number | null>(null);
  const shrinkTimer = useRef<number | null>(null);
  // True while the native window is dashboard-sized, until the shrink after a close has run.
  const windowIsLarge = useRef(false);
  typingRef.current = typing;
  onHoverRef.current = onHover;

  const goTo = useCallback((next: NotchStage) => {
    stageRef.current = next;
    setStage(next);
  }, []);

  // A hover must only cancel the pending close. If it cancelled the pending window shrink too,
  // the native window would stay dashboard-sized over the page while the UI shows nothing.
  const cancelClose = useCallback(() => {
    if (closeTimer.current !== null) window.clearTimeout(closeTimer.current);
    closeTimer.current = null;
  }, []);

  const cancelAll = useCallback(() => {
    cancelClose();
    if (shrinkTimer.current !== null) window.clearTimeout(shrinkTimer.current);
    shrinkTimer.current = null;
  }, [cancelClose]);

  const open = useCallback(() => {
    cancelAll();
    windowIsLarge.current = true;
    goTo("expanded");
    callQuietly("set_notch_expanded", { expanded: true });
  }, [cancelAll, goTo]);

  const close = useCallback(() => {
    cancelClose();
    goTo("idle");
    // Shrink the native window once the retract has finished. A shrink that is already pending
    // is left alone.
    if (windowIsLarge.current && shrinkTimer.current === null) {
      shrinkTimer.current = window.setTimeout(() => {
        shrinkTimer.current = null;
        windowIsLarge.current = false;
        callQuietly("set_notch_expanded", { expanded: false });
      }, WINDOW_SHRINK_DELAY_MS);
    }
  }, [cancelClose, goTo]);

  const hoverStart = useCallback(() => {
    cancelClose();
    if (stageRef.current === "idle") goTo("peek");
    onHoverRef.current();
  }, [cancelClose, goTo]);

  const hoverEnd = useCallback(() => {
    cancelClose();
    if (stageRef.current === "expanded" && typingRef.current) return;
    closeTimer.current = window.setTimeout(close, CLOSE_DELAY_MS);
  }, [cancelClose, close]);

  useNativeEvent<boolean>("notch://hover", (hovered) => (hovered ? hoverStart() : hoverEnd()));
  useNativeEvent<null>("notch://open", open);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && stageRef.current === "expanded") close();
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      cancelAll();
    };
  }, [close, cancelAll]);

  // The panel only takes keyboard focus while the Setup field is on screen. Everywhere else,
  // typing must stay with the app you were using.
  useEffect(() => {
    callQuietly("set_keyboard_focus", { allow: stage === "expanded" && typing });
  }, [stage, typing]);

  return { stage, open, close, hoverStart, hoverEnd };
}
