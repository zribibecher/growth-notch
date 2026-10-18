import { formatCompactNumber, timeAgo } from "../lib/format";
import { FALLBACK_HERO, METRIC_CATALOG } from "../lib/metrics";
import type { Theme } from "../lib/theme";
import type { MetricKey, YouTubeMetrics } from "../types/telemetry";
import { Icon } from "./Icon";
import { RemoteImage } from "./RemoteImage";

interface Props {
  theme: Theme;
  youtube: YouTubeMetrics | null;
  /** Which metrics the user picked on the Metrics tab. */
  enabled: MetricKey[];
  subscriberDelta: number;
  now: number;
  lastUpdated: number;
  error: string | null;
  refreshing: boolean;
  onRefresh: () => void;
  onToggleTheme: () => void;
}

/** The main view: one big number, two supporting stats, quiet chips, and the latest upload. */
export function PulseTab({
  theme: t,
  youtube: yt,
  enabled,
  subscriberDelta,
  now,
  lastUpdated,
  error,
  refreshing,
  onRefresh,
  onToggleTheme,
}: Props) {
  const picked = METRIC_CATALOG.filter((metric) => enabled.includes(metric.key));
  const hero = picked.find((metric) => metric.key === "subscribers") ?? picked[0] ?? FALLBACK_HERO;
  const others = picked.filter((metric) => metric !== hero);
  const stats = others.slice(0, 2);
  const chips = others.slice(2);

  return (
    <div className="flex flex-col gap-2.5 flex-1 min-h-0 tabular-nums">
      <div className="flex items-end justify-between gap-3">
        <div className="min-w-0">
          <div className={`text-[9px] uppercase tracking-[0.14em] font-semibold ${t.muted}`}>
            {hero.label}
          </div>
          <div className="flex items-baseline gap-2">
            <div
              data-testid={`card-${hero.key}`}
              className={`text-[36px] leading-[1.05] font-semibold tracking-tight ${t.strong}`}
            >
              {hero.value(yt)}
            </div>
            {hero.key === "subscribers" && subscriberDelta !== 0 && (
              <span
                data-testid="subs-delta"
                title="Change since the app launched (YouTube rounds public counts)"
                className={`text-[10.5px] font-semibold px-1.5 py-0.5 rounded-full ${
                  subscriberDelta > 0
                    ? "bg-emerald-500/15 text-emerald-400"
                    : "bg-rose-500/15 text-rose-400"
                }`}
              >
                {subscriberDelta > 0 ? "+" : ""}
                {formatCompactNumber(subscriberDelta)}
              </span>
            )}
          </div>
        </div>

        <div className="flex items-stretch gap-3 shrink-0">
          {stats.map((metric) => (
            <div key={metric.key} className={`pl-3 border-l min-w-[84px] ${t.hairline}`}>
              <div
                className={`text-[9px] uppercase tracking-[0.14em] font-semibold whitespace-nowrap ${t.muted}`}
              >
                {metric.shortLabel ?? metric.label}
              </div>
              <div
                data-testid={`card-${metric.key}`}
                className={`text-[20px] leading-tight font-semibold tracking-tight ${t.tone(metric)}`}
              >
                {metric.value(yt)}
              </div>
              {metric.hint && (
                <div className={`text-[9px] leading-tight ${t.muted}`}>{metric.hint}</div>
              )}
            </div>
          ))}
        </div>
      </div>

      {chips.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {chips.map((metric) => (
            <span
              key={metric.key}
              className={`inline-flex items-baseline gap-1.5 px-2 py-0.5 rounded-full border text-[10.5px] ${t.glass}`}
            >
              <span className={t.muted}>{metric.label}</span>
              <strong data-testid={`card-${metric.key}`} className={t.tone(metric)}>
                {metric.value(yt)}
              </strong>
            </span>
          ))}
        </div>
      )}

      {enabled.includes("latest_upload_bar") && (
        <LatestUpload theme={t} youtube={yt} now={now} imageVersion={lastUpdated} />
      )}

      <div className="mt-auto flex items-center justify-between text-[9.5px] -mb-1">
        {error ? (
          <span
            data-testid="status-msg"
            role="status"
            aria-live="polite"
            className={`${t.warning} truncate`}
          >
            {error}
          </span>
        ) : (
          <span className={t.muted}>
            {lastUpdated ? `Updated ${timeAgo(lastUpdated, now)}` : "Waiting for first update"}
          </span>
        )}
        <span className="flex items-center gap-1">
          <button
            type="button"
            data-testid="theme-toggle-btn"
            aria-label="Toggle light or dark theme"
            title={t.dark ? "Switch to Light Theme" : "Switch to Dark Theme"}
            onClick={(e) => {
              e.stopPropagation();
              onToggleTheme();
            }}
            className={`${t.iconButton} !w-5 !h-5`}
          >
            <Icon name={t.dark ? "sun" : "moon"} />
          </button>
          <button
            type="button"
            data-testid="refresh-btn"
            aria-label="Refresh now"
            title="Refresh now"
            disabled={refreshing}
            onClick={(e) => {
              e.stopPropagation();
              onRefresh();
            }}
            className={`${t.iconButton} !w-5 !h-5 disabled:opacity-40 ${refreshing ? "animate-spin" : ""}`}
          >
            <Icon name="refresh" />
          </button>
        </span>
      </div>
    </div>
  );
}

interface LatestUploadProps {
  theme: Theme;
  youtube: YouTubeMetrics | null;
  now: number;
  imageVersion: number;
}

function LatestUpload({ theme: t, youtube: yt, now, imageVersion }: LatestUploadProps) {
  const age = yt?.latestVideoPublishedAt
    ? ` · ${timeAgo(Date.parse(yt.latestVideoPublishedAt), now)}`
    : "";
  return (
    <div className={`flex items-center gap-3 p-2 rounded-xl border ${t.glass}`}>
      <RemoteImage
        src={yt?.latestVideoThumbnail}
        version={imageVersion}
        className="w-[72px] h-[40px] rounded-md object-cover shrink-0"
        fallback={
          <div
            aria-hidden="true"
            className={`w-[72px] h-[40px] rounded-md shrink-0 ${t.dark ? "bg-white/[0.06]" : "bg-slate-100"}`}
          />
        }
      />
      <div className="min-w-0 flex-1">
        <div className={`text-[9px] uppercase tracking-[0.14em] font-semibold ${t.muted}`}>
          Latest upload{age}
        </div>
        <div
          data-testid="latest-video-title"
          className={`text-[12px] font-medium truncate ${t.strong}`}
        >
          {yt?.latestVideoTitle ?? "No recent uploads fetched yet"}
        </div>
        <div className={`flex items-center gap-3 text-[10.5px] ${t.muted}`}>
          <span className="inline-flex items-center gap-1">
            <Icon name="eye" />
            <strong className={t.strong}>{formatCompactNumber(yt?.latestVideoViews)}</strong>
          </span>
          <span className="inline-flex items-center gap-1">
            <Icon name="heart" className="w-2.5 h-2.5 fill-rose-500" />
            <strong className={t.strong}>{formatCompactNumber(yt?.latestVideoLikes)}</strong>
          </span>
          <span className="inline-flex items-center gap-1">
            <Icon name="comment" className="w-2.5 h-2.5 fill-sky-400" />
            <strong className={t.strong}>{formatCompactNumber(yt?.latestVideoComments)}</strong>
          </span>
        </div>
      </div>
    </div>
  );
}
