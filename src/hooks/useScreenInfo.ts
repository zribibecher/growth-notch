import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { DEFAULT_SCREEN, isScreenInfo } from "../lib/screen";
import type { ScreenInfo } from "../types/telemetry";
import { useNativeEvent } from "./useNativeEvent";

/** The screen layout from the native side: read once at start, then kept current by events. */
export function useScreenInfo(): ScreenInfo {
  const [info, setInfo] = useState<ScreenInfo>(DEFAULT_SCREEN);

  useNativeEvent<unknown>("notch://layout", (next) => {
    if (isScreenInfo(next)) setInfo(next);
  });

  useEffect(() => {
    let disposed = false;
    Promise.resolve(invoke<unknown>("get_screen_info"))
      .then((next) => {
        if (isScreenInfo(next) && !disposed) setInfo(next);
      })
      .catch(() => {
        // Outside the desktop app the default layout is right.
      });
    return () => {
      disposed = true;
    };
  }, []);

  return info;
}
