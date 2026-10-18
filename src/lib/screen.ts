import type { CSSProperties } from "react";
import type { NotchStage } from "../types/notch";
import type { ScreenInfo } from "../types/telemetry";

/** Used until the desktop app reports the real screen, and in the browser and tests. */
export const DEFAULT_SCREEN: ScreenInfo = {
  metrics: { hasNotch: false, notchWidth: 0, notchHeight: 0, menuBarHeight: 24 },
  layout: {
    peekWidth: 248,
    peekHeight: 32,
    expandedInsetTop: 0,
    idleWidth: 132,
    idleHeight: 24,
    wings: false,
  },
};

/** Guards against a malformed payload so a bad event can never crash the pill. */
export function isScreenInfo(value: unknown): value is ScreenInfo {
  const info = value as Partial<ScreenInfo> | null | undefined;
  return !!info && !!info.metrics && !!info.layout && typeof info.layout.peekWidth === "number";
}

/** Height of the open dashboard before the camera inset. Keep in sync with `EXPANDED_PILL_HEIGHT` in notch.rs. */
const EXPANDED_HEIGHT = 216;

/**
 * Next to a hardware notch the pill hugs the camera housing, so its size comes from the screen.
 * Without one, the Tailwind sizes on the container apply and no inline style is needed.
 */
export function pillStyle(screen: ScreenInfo, stage: NotchStage): CSSProperties | undefined {
  const { layout, metrics } = screen;
  if (!layout.wings) return undefined;
  if (stage === "idle") return { width: metrics.notchWidth, height: metrics.notchHeight };
  if (stage === "peek") return { width: layout.peekWidth, height: layout.peekHeight };
  return {
    height: EXPANDED_HEIGHT + layout.expandedInsetTop,
    paddingTop: layout.expandedInsetTop + 8,
  };
}
