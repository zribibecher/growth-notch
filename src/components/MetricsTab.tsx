import { METRIC_CATALOG } from "../lib/metrics";
import type { Theme } from "../lib/theme";
import type { MetricKey } from "../types/telemetry";

interface Props {
  theme: Theme;
  enabled: MetricKey[];
  onToggle: (key: MetricKey) => void;
}

/** Everything that can show on Pulse, as a grid of switches. */
export function MetricsTab({ theme: t, enabled, onToggle }: Props) {
  const options = [
    ...METRIC_CATALOG.map(({ key, label, shortLabel, hint }) => ({
      key,
      label: shortLabel ?? label,
      hint,
    })),
    { key: "latest_upload_bar" as const, label: "Latest upload", hint: undefined },
  ];

  return (
    <div className="flex flex-col gap-2 flex-1 min-h-0">
      <div className={`text-[11px] ${t.muted}`}>
        Pick what shows on Pulse. The first three become headlines.
      </div>
      <div className="grid grid-cols-3 gap-1.5 flex-1 min-h-0 overflow-y-auto pr-0.5 content-start">
        {options.map(({ key, label, hint }) => {
          const on = enabled.includes(key);
          return (
            <button
              key={key}
              type="button"
              role="switch"
              aria-checked={on}
              data-testid={`toggle-metric-${key}`}
              title={hint}
              onClick={(e) => {
                e.stopPropagation();
                onToggle(key);
              }}
              className={t.metricToggle(on)}
            >
              <span className="truncate">{label}</span>
              <span aria-hidden="true">{on ? "✓" : "+"}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}
