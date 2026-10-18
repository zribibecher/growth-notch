import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";
import type { CredentialKind } from "../lib/credentials";
import type { CredentialStatus, TelemetrySnapshot } from "../types/telemetry";
import { useNativeEvent } from "./useNativeEvent";

const POLL_MS = 60_000;
/** How often "5m ago" labels are recalculated. */
const CLOCK_MS = 15_000;
/** Brushing the cursor over the pill refreshes at most this often. */
const HOVER_REFRESH_COOLDOWN_MS = 15_000;

const NO_CREDENTIALS: CredentialStatus = {
  youtubeConfigured: false,
  youtubeChannelConfigured: false,
};

/** The channel numbers, the saved-credential status, and everything that changes them. */
export function useTelemetry() {
  const [snapshot, setSnapshot] = useState<TelemetrySnapshot | null>(null);
  const [credentials, setCredentials] = useState<CredentialStatus>(NO_CREDENTIALS);
  const [credentialsLoaded, setCredentialsLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [now, setNow] = useState(() => Date.now());
  // Subscribers when the app first heard from YouTube, to show how far they moved since.
  const [baselineSubscribers, setBaselineSubscribers] = useState<number | null>(null);
  const lastHoverRefresh = useRef(0);

  const refresh = useCallback(async () => {
    setRefreshing(true);
    try {
      const [nextSnapshot, nextCredentials] = await Promise.all([
        invoke<TelemetrySnapshot>("refresh_telemetry"),
        invoke<CredentialStatus>("get_credential_status"),
      ]);
      setSnapshot(nextSnapshot);
      setCredentials(nextCredentials);
      setCredentialsLoaded(true);
      setError(nextSnapshot?.errors?.youtube ?? null);
      setNow(Date.now());
      const subscribers = nextSnapshot?.youtube?.subscriberCount;
      if (subscribers !== undefined) {
        setBaselineSubscribers((previous) => previous ?? subscribers);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setRefreshing(false);
    }
  }, []);

  const refreshOnHover = useCallback(() => {
    const t = Date.now();
    if (t - lastHoverRefresh.current >= HOVER_REFRESH_COOLDOWN_MS) {
      lastHoverRefresh.current = t;
      void refresh();
    }
  }, [refresh]);

  useEffect(() => {
    void refresh();
    const poll = window.setInterval(() => void refresh(), POLL_MS);
    const clock = window.setInterval(() => setNow(Date.now()), CLOCK_MS);
    return () => {
      window.clearInterval(poll);
      window.clearInterval(clock);
    };
  }, [refresh]);

  useNativeEvent<null>("notch://refresh", () => void refresh());

  const saveCredential = useCallback(
    async (provider: CredentialKind, value: string) => {
      try {
        setError(null);
        setCredentials(await invoke<CredentialStatus>("save_api_key", { provider, apiKey: value }));
        void refresh();
      } catch (err) {
        setError(String(err));
      }
    },
    [refresh],
  );

  const clearCredentials = useCallback(async () => {
    try {
      setError(null);
      await invoke<CredentialStatus>("delete_api_key", { provider: "youtube_api_key" });
      setCredentials(
        await invoke<CredentialStatus>("delete_api_key", { provider: "youtube_channel_id" }),
      );
      void refresh();
    } catch (err) {
      setError(String(err));
    }
  }, [refresh]);

  const youtube = snapshot?.youtube ?? null;
  return {
    youtube,
    lastUpdated: snapshot?.lastUpdatedUnixMs ?? 0,
    credentials,
    /** True once we know there is no API key or channel ID saved yet. */
    needsSetup:
      credentialsLoaded &&
      (!credentials.youtubeConfigured || !credentials.youtubeChannelConfigured),
    error,
    refreshing,
    now,
    subscriberDelta:
      youtube && baselineSubscribers !== null ? youtube.subscriberCount - baselineSubscribers : 0,
    refresh,
    refreshOnHover,
    saveCredential,
    clearCredentials,
  };
}
