import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";

/** Runs `handler` for every native event called `name` while the component is mounted. */
export function useNativeEvent<T>(name: string, handler: (payload: T) => void): void {
  // Always call the latest handler without resubscribing every render.
  const latest = useRef(handler);
  latest.current = handler;

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    Promise.resolve(listen<T>(name, (event) => latest.current(event.payload)))
      .then((stop) => {
        // The component may unmount before listen() resolves; stop right away in that case.
        if (disposed) stop();
        else unlisten = stop;
      })
      .catch(() => {
        // Outside the desktop app (a browser, the tests) there are no native events.
      });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [name]);
}
