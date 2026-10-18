import type { Theme } from "../lib/theme";
import type { TabId } from "../types/notch";
import type { YouTubeMetrics } from "../types/telemetry";
import { Icon } from "./Icon";
import { RemoteImage } from "./RemoteImage";

const TABS: { id: TabId; testId: string; label: string }[] = [
  { id: "pulse", testId: "pulse-tab-btn", label: "Pulse" },
  { id: "metrics", testId: "metrics-tab-btn", label: "Metrics" },
  { id: "setup", testId: "setup-tab-btn", label: "Setup" },
];

function handleOf(youtube: YouTubeMetrics | null): string {
  if (!youtube) return "Not connected";
  if (!youtube.customUrl) return youtube.channelId;
  return youtube.customUrl.startsWith("@") ? youtube.customUrl : `@${youtube.customUrl}`;
}

interface Props {
  theme: Theme;
  youtube: YouTubeMetrics | null;
  imageVersion: number;
  tab: TabId;
  onTab: (tab: TabId) => void;
  onClose: () => void;
}

/** Channel identity on the left; tabs and the close button on the right. */
export function Header({ theme: t, youtube, imageVersion, tab, onTab, onClose }: Props) {
  return (
    <div className={`flex items-center justify-between gap-2 pb-2 border-b ${t.hairline}`}>
      <div className="flex items-center gap-2 min-w-0">
        <RemoteImage
          src={youtube?.avatarUrl}
          version={imageVersion}
          className="w-5 h-5 rounded-full object-cover shrink-0"
        />
        <div className="min-w-0 leading-tight">
          <div
            className={`text-[12px] font-semibold truncate ${t.strong}`}
            title={youtube?.channelTitle ?? undefined}
          >
            {youtube?.channelTitle ?? "Growth Notch"}
          </div>
          <div data-testid="channel-handle" className={`text-[10px] truncate ${t.muted}`}>
            {handleOf(youtube)}
          </div>
        </div>
      </div>

      <div
        className="flex items-center gap-0.5 shrink-0"
        role="tablist"
        aria-label="Dashboard sections"
      >
        {TABS.map(({ id, testId, label }) => (
          <button
            key={id}
            type="button"
            role="tab"
            aria-selected={tab === id}
            data-testid={testId}
            onClick={(e) => {
              e.stopPropagation();
              onTab(id);
            }}
            className={t.tab(tab === id)}
          >
            {label}
          </button>
        ))}
        <span className={`mx-1 h-3 w-px ${t.dark ? "bg-white/10" : "bg-slate-300"}`} />
        <button
          type="button"
          data-testid="collapse-btn"
          aria-label="Close dashboard"
          title="Close"
          onClick={(e) => {
            e.stopPropagation();
            onClose();
          }}
          className={t.iconButton}
        >
          <Icon name="close" />
        </button>
      </div>
    </div>
  );
}
