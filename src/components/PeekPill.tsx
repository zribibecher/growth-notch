import { formatCompactNumber } from "../lib/format";
import type { LayoutHints, YouTubeMetrics } from "../types/telemetry";
import { Icon } from "./Icon";

interface Props {
  /** True while the pill is in its peek stage. */
  visible: boolean;
  needsSetup: boolean;
  youtube: YouTubeMetrics | null;
  layout: LayoutHints;
  /** Width of the camera housing the stats must keep clear of, if there is one. */
  notchWidth: number;
}

/** The compact pill shown on hover: subscribers, likes and comments. */
export function PeekPill({ visible, needsSetup, youtube, layout, notchWidth }: Props) {
  const { wings } = layout;
  const gap = <div style={{ width: notchWidth }} aria-hidden="true" />;

  return (
    <div
      data-testid="peek-pill-layer"
      aria-hidden={!visible}
      style={wings ? { height: layout.peekHeight } : undefined}
      className={`notch-pill-layer absolute top-0 flex items-center justify-between text-[11.5px] font-medium tabular-nums tracking-tight ${
        wings ? "inset-x-2 h-[32px]" : "inset-x-4 h-[30px]"
      } ${
        visible
          ? "opacity-100 scale-100 translate-y-0 pointer-events-auto"
          : "opacity-0 scale-95 -translate-y-1 pointer-events-none"
      }`}
    >
      {needsSetup ? (
        wings ? (
          <>
            <span className="flex-1 text-center text-white/70 text-[11px]">Connect</span>
            {gap}
            <span className="flex-1 text-center text-white/70 text-[11px]">your channel</span>
          </>
        ) : (
          <span className="w-full text-center text-white/70 text-[11px]">
            Click to connect your channel
          </span>
        )
      ) : (
        <>
          <div
            className={`flex items-center gap-1.5 ${wings ? "flex-1 justify-center" : ""}`}
            title="Subscribers"
          >
            <span className="relative flex h-1.5 w-1.5">
              <span
                className={`absolute inline-flex h-full w-full rounded-full bg-red-500 opacity-60 ${
                  visible ? "animate-ping" : ""
                }`}
              />
              <span className="relative inline-flex h-1.5 w-1.5 rounded-full bg-red-500" />
            </span>
            <strong data-testid="compact-subs" className="font-semibold text-white">
              {formatCompactNumber(youtube?.subscriberCount)}
            </strong>
            <span className="text-[10px] text-white/45">subs</span>
          </div>
          {wings && gap}
          <div
            className={`flex items-center ${wings ? "flex-1 justify-center gap-3" : "contents"}`}
          >
            <div className="flex items-center gap-1" title="Likes across your last 10 uploads">
              <Icon name="heart" className="w-3 h-3 fill-rose-500" />
              <strong data-testid="compact-likes" className="font-semibold text-white">
                {formatCompactNumber(youtube?.likeCount)}
              </strong>
            </div>
            <div className="flex items-center gap-1" title="Comments across your last 10 uploads">
              <Icon name="comment" className="w-3 h-3 fill-sky-400" />
              <strong data-testid="compact-comments" className="font-semibold text-white">
                {formatCompactNumber(youtube?.commentCount)}
              </strong>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
