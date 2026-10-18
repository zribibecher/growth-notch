import { describe, expect, it } from "vitest";
import { classifyCredential } from "../lib/credentials";
import { formatCompactNumber, timeAgo } from "../lib/format";
import { isTrustedImageUrl } from "../lib/images";
import { METRIC_CATALOG } from "../lib/metrics";
import { DEFAULT_SCREEN, isScreenInfo, pillStyle } from "../lib/screen";
import { makeTheme } from "../lib/theme";
import type { YouTubeMetrics } from "../types/telemetry";

const channel = {
  subscriberCount: 1500,
  videoCount: 10,
  viewCount: 50_000,
  likeCount: 900,
  commentCount: 100,
  latestVideoViews: 2_300,
  latestVideoLikes: 120,
  latestVideoComments: 8,
  country: "TN",
  publishedAt: "2019-04-02T10:00:00Z",
} as YouTubeMetrics;

const valueOf = (key: string, yt: YouTubeMetrics | null) =>
  METRIC_CATALOG.find((metric) => metric.key === key)!.value(yt);

describe("formatCompactNumber", () => {
  it("shortens thousands and millions to one decimal", () => {
    expect(formatCompactNumber(842)).toBe("842");
    expect(formatCompactNumber(1_500)).toBe("1.5K");
    expect(formatCompactNumber(8_910_200)).toBe("8.9M");
    expect(formatCompactNumber(null)).toBe("—");
    expect(formatCompactNumber(undefined)).toBe("—");
  });
});

describe("timeAgo", () => {
  const now = 1_000_000_000_000;
  it("buckets seconds, minutes, hours and days", () => {
    expect(timeAgo(now - 3_000, now)).toBe("just now");
    expect(timeAgo(now - 42_000, now)).toBe("42s ago");
    expect(timeAgo(now - 5 * 60_000, now)).toBe("5m ago");
    expect(timeAgo(now - 3 * 3_600_000, now)).toBe("3h ago");
    expect(timeAgo(now - 2 * 86_400_000, now)).toBe("2d ago");
    expect(timeAgo(Number.NaN, now)).toBe("");
  });
});

describe("classifyCredential", () => {
  it("tells API keys from channel IDs and rejects everything else", () => {
    expect(classifyCredential("AIzaSyExampleKeyExampleKeyExample123")).toBe("youtube_api_key");
    expect(classifyCredential("  UCabcdefghijklmnopqrstuv  ")).toBe("youtube_channel_id");
    expect(classifyCredential("@techuntangled")).toBeNull();
    expect(classifyCredential("UCtooShort")).toBeNull();
    expect(classifyCredential("")).toBeNull();
  });
});

describe("isTrustedImageUrl", () => {
  it("accepts only plain HTTPS URLs on YouTube image hosts", () => {
    expect(isTrustedImageUrl("https://i.ytimg.com/vi/abc/mqdefault.jpg")).toBe(true);
    expect(isTrustedImageUrl("https://yt3.ggpht.com/a=s88")).toBe(true);
    expect(isTrustedImageUrl("https://yt3.googleusercontent.com/a")).toBe(true);
    for (const bad of [
      "http://i.ytimg.com/a.jpg",
      "https://evil.example.com/a.png",
      "https://i.ytimg.com.evil.com/a.png",
      "https://user:pass@i.ytimg.com/a.png",
      "https://i.ytimg.com:8443/a.png",
      "https://i.ytimg.com/../etc/passwd",
      "javascript:alert(1)",
      "not a url",
      "",
      null,
      undefined,
    ]) {
      expect(isTrustedImageUrl(bad)).toBe(false);
    }
  });
});

describe("metric catalog", () => {
  it("derives averages, engagement and the join year from the channel", () => {
    expect(valueOf("avg_views", channel)).toBe("5.0K");
    expect(valueOf("engagement_rate", channel)).toBe("2.0%");
    expect(valueOf("joined_year", channel)).toBe("2019");
    expect(valueOf("country", channel)).toBe("TN");
    expect(valueOf("latest_views", channel)).toBe("2.3K");
  });

  it("shows a dash instead of dividing by zero or guessing", () => {
    const empty = {
      ...channel,
      videoCount: 0,
      viewCount: 0,
      country: undefined,
      publishedAt: undefined,
    };
    for (const key of ["avg_views", "engagement_rate", "country", "joined_year"]) {
      expect(valueOf(key, empty)).toBe("—");
      expect(valueOf(key, null)).toBe("—");
    }
  });

  it("has one entry per key, and labels the last-10 metrics as such", () => {
    const keys = METRIC_CATALOG.map((metric) => metric.key);
    expect(new Set(keys).size).toBe(keys.length);
    for (const key of ["likes", "comments"]) {
      expect(METRIC_CATALOG.find((metric) => metric.key === key)?.hint).toBe("last 10 uploads");
    }
  });
});

describe("pill layout", () => {
  const notched = {
    metrics: { hasNotch: true, notchWidth: 185, notchHeight: 32, menuBarHeight: 37 },
    layout: {
      peekWidth: 341,
      peekHeight: 32,
      expandedInsetTop: 32,
      idleWidth: 185,
      idleHeight: 32,
      wings: true,
    },
  };

  it("lets Tailwind size the pill when there is no notch", () => {
    for (const stage of ["idle", "peek", "expanded"] as const) {
      expect(pillStyle(DEFAULT_SCREEN, stage)).toBeUndefined();
    }
  });

  it("sizes the pill from the screen when it wraps a hardware notch", () => {
    expect(pillStyle(notched, "idle")).toEqual({ width: 185, height: 32 });
    expect(pillStyle(notched, "peek")).toEqual({ width: 341, height: 32 });
    expect(pillStyle(notched, "expanded")).toEqual({ height: 248, paddingTop: 40 });
  });

  it("recognises a real screen payload and rejects nonsense", () => {
    expect(isScreenInfo(notched)).toBe(true);
    for (const bad of [null, undefined, {}, { metrics: {}, layout: {} }, "screen", 42]) {
      expect(isScreenInfo(bad)).toBe(false);
    }
  });
});

describe("theme", () => {
  it("switches every colour between dark and light", () => {
    const dark = makeTheme(true);
    const light = makeTheme(false);
    expect(dark.strong).not.toBe(light.strong);
    expect(dark.muted).not.toBe(light.muted);
    expect(dark.tab(true)).not.toBe(light.tab(true));
    expect(dark.tab(true)).not.toBe(dark.tab(false));
    expect(dark.statusChip(true)).not.toBe(dark.statusChip(false));
    expect(dark.metricToggle(true)).not.toBe(dark.metricToggle(false));
  });

  it("gives a metric its own colour, and plain text when it has none", () => {
    const theme = makeTheme(true);
    const likes = METRIC_CATALOG.find((metric) => metric.key === "likes")!;
    const subscribers = METRIC_CATALOG.find((metric) => metric.key === "subscribers")!;
    expect(theme.tone(likes)).toBe("text-rose-400");
    expect(theme.tone(subscribers)).toBe(theme.strong);
    expect(makeTheme(false).tone(likes)).toBe("text-rose-600");
  });
});
