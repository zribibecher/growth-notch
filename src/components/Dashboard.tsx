import type { Theme } from "../lib/theme";
import type { TabId } from "../types/notch";
import type { MetricKey } from "../types/telemetry";
import type { useTelemetry } from "../hooks/useTelemetry";
import { Header } from "./Header";
import { MetricsTab } from "./MetricsTab";
import { PulseTab } from "./PulseTab";
import { SetupTab } from "./SetupTab";
import { Welcome } from "./Welcome";

interface Props {
  theme: Theme;
  /** True while the pill is in its expanded stage. */
  visible: boolean;
  telemetry: ReturnType<typeof useTelemetry>;
  tab: TabId;
  onTab: (tab: TabId) => void;
  onClose: () => void;
  enabledMetrics: MetricKey[];
  onToggleMetric: (key: MetricKey) => void;
  onToggleTheme: () => void;
}

/** The studio dashboard: a header, then whichever tab is selected. */
export function Dashboard({
  theme,
  visible,
  telemetry,
  tab,
  onTab,
  onClose,
  enabledMetrics,
  onToggleMetric,
  onToggleTheme,
}: Props) {
  return (
    <div
      data-testid="expanded-dashboard-layer"
      aria-hidden={!visible}
      className={`notch-expanded-layer flex flex-col gap-2.5 h-full w-full ${
        visible
          ? "opacity-100 scale-100 translate-y-0 pointer-events-auto"
          : "opacity-0 scale-[0.96] -translate-y-1.5 pointer-events-none"
      }`}
    >
      <Header
        theme={theme}
        youtube={telemetry.youtube}
        imageVersion={telemetry.lastUpdated}
        tab={tab}
        onTab={onTab}
        onClose={onClose}
      />

      {tab === "pulse" &&
        (telemetry.needsSetup ? (
          <Welcome theme={theme} onStart={() => onTab("setup")} />
        ) : (
          <PulseTab
            theme={theme}
            youtube={telemetry.youtube}
            enabled={enabledMetrics}
            subscriberDelta={telemetry.subscriberDelta}
            now={telemetry.now}
            lastUpdated={telemetry.lastUpdated}
            error={telemetry.error}
            refreshing={telemetry.refreshing}
            onRefresh={() => void telemetry.refresh()}
            onToggleTheme={onToggleTheme}
          />
        ))}

      {tab === "metrics" && (
        <MetricsTab theme={theme} enabled={enabledMetrics} onToggle={onToggleMetric} />
      )}

      {tab === "setup" && (
        <SetupTab
          theme={theme}
          credentials={telemetry.credentials}
          error={telemetry.error}
          onSave={(kind, value) => void telemetry.saveCredential(kind, value)}
          onClear={() => void telemetry.clearCredentials()}
        />
      )}
    </div>
  );
}
