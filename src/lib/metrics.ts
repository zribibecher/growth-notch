import type { MetricKey, YouTubeMetrics } from "../types/telemetry";
import { formatCompactNumber } from "./format";

type Youtube = YouTubeMetrics | null | undefined;

export interface MetricDefinition {
  key: MetricKey;
  label: string;
  /** Used where space is tight: the stat columns and the picker. */
  shortLabel?: string;
  /** A caption under the value. */
  hint?: string;
  /** Text colour per theme, so numbers can be told apart at a glance. Plain text if missing. */
  tone?: { dark: string; light: string };
  value: (youtube: Youtube) => string;
}

const LAST_TEN = "last 10 uploads";

const subscribers: MetricDefinition = {
  key: "subscribers",
  label: "Subscribers",
  value: (yt) => formatCompactNumber(yt?.subscriberCount),
};

export const METRIC_CATALOG: MetricDefinition[] = [
  subscribers,
  {
    key: "likes",
    label: "Likes · last 10",
    shortLabel: "Likes",
    hint: LAST_TEN,
    tone: { dark: "text-rose-400", light: "text-rose-600" },
    value: (yt) => formatCompactNumber(yt?.likeCount),
  },
  {
    key: "comments",
    label: "Comments · last 10",
    shortLabel: "Comments",
    hint: LAST_TEN,
    tone: { dark: "text-sky-400", light: "text-sky-600" },
    value: (yt) => formatCompactNumber(yt?.commentCount),
  },
  {
    key: "views",
    label: "Total Views",
    tone: { dark: "text-emerald-400", light: "text-emerald-600" },
    value: (yt) => formatCompactNumber(yt?.viewCount),
  },
  {
    key: "videos",
    label: "Videos",
    tone: { dark: "text-violet-400", light: "text-violet-600" },
    value: (yt) => formatCompactNumber(yt?.videoCount),
  },
  {
    key: "avg_views",
    label: "Avg Views",
    tone: { dark: "text-amber-300", light: "text-amber-600" },
    value: (yt) =>
      yt && yt.videoCount > 0 ? formatCompactNumber(Math.round(yt.viewCount / yt.videoCount)) : "—",
  },
  {
    key: "engagement_rate",
    label: "Engagement",
    tone: { dark: "text-teal-300", light: "text-teal-600" },
    value: (yt) =>
      yt && yt.viewCount > 0
        ? `${(((yt.likeCount + yt.commentCount) / yt.viewCount) * 100).toFixed(1)}%`
        : "—",
  },
  {
    key: "latest_views",
    label: "Latest Views",
    tone: { dark: "text-indigo-300", light: "text-indigo-600" },
    value: (yt) => formatCompactNumber(yt?.latestVideoViews),
  },
  {
    key: "latest_likes",
    label: "Latest Likes",
    tone: { dark: "text-rose-400", light: "text-rose-600" },
    value: (yt) => formatCompactNumber(yt?.latestVideoLikes),
  },
  {
    key: "latest_comments",
    label: "Latest Comments",
    tone: { dark: "text-sky-400", light: "text-sky-600" },
    value: (yt) => formatCompactNumber(yt?.latestVideoComments),
  },
  {
    key: "country",
    label: "Country",
    value: (yt) => yt?.country ?? "—",
  },
  {
    key: "joined_year",
    label: "Joined",
    value: (yt) => (yt?.publishedAt ? new Date(yt.publishedAt).getFullYear().toString() : "—"),
  },
];

/** What shows on Pulse until the user picks something else. */
export const DEFAULT_METRICS: MetricKey[] = [
  "subscribers",
  "likes",
  "comments",
  "latest_upload_bar",
];

/** The headline when nothing is selected. */
export const FALLBACK_HERO = subscribers;
