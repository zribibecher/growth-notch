import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Dashboard } from "./components/Dashboard";
import { PeekPill } from "./components/PeekPill";
import { useNotch } from "./hooks/useNotch";
import { useScreenInfo } from "./hooks/useScreenInfo";
import { useTelemetry } from "./hooks/useTelemetry";
import { DEFAULT_METRICS } from "./lib/metrics";
import { pillStyle } from "./lib/screen";
import { makeTheme } from "./lib/theme";
import type { NotchStage, TabId } from "./types/notch";
import type { MetricKey } from "./types/telemetry";

/** Size, corner radius and opacity of the pill in each stage. Idle is invisible until the cursor arrives. */
const SHAPE: Record<NotchStage, string> = {
  idle: "w-[132px] h-[24px] rounded-b-[12px] opacity-0 cursor-pointer",
  peek: "w-[248px] h-[32px] rounded-b-[16px] opacity-100 cursor-pointer",
  expanded: "w-[400px] h-[216px] rounded-b-[24px] opacity-100 px-4 pt-2 pb-3",
};

/** The concave fillets either side of the pill that blend it into the top edge of the screen. */
function Ear({ side, hidden }: { side: "left" | "right"; hidden: boolean }) {
  return (
    <div
      aria-hidden="true"
      className={`notch-ear-${side} w-[8px] h-[8px] shrink-0 pointer-events-none transition-opacity duration-200 ${
        hidden ? "opacity-0" : "opacity-100"
      }`}
    />
  );
}

export default function App() {
  const screen = useScreenInfo();
  const telemetry = useTelemetry();
  const [tab, setTab] = useState<TabId>("pulse");
  const [dark, setDark] = useState(true);
  const [enabledMetrics, setEnabledMetrics] = useState<MetricKey[]>(DEFAULT_METRICS);
  const { stage, open, close, hoverStart, hoverEnd } = useNotch({
    typing: tab === "setup",
    onHover: telemetry.refreshOnHover,
  });
  const theme = useMemo(() => makeTheme(dark), [dark]);

  // Idle is invisible, so a first-time user would see nothing: open the welcome card once.
  const welcomed = useRef(false);
  useEffect(() => {
    if (telemetry.needsSetup && !welcomed.current) {
      welcomed.current = true;
      open();
    }
  }, [telemetry.needsSetup, open]);

  const toggleMetric = useCallback((key: MetricKey) => {
    setEnabledMetrics((current) =>
      current.includes(key) ? current.filter((item) => item !== key) : [...current, key],
    );
  }, []);

  const idle = stage === "idle";

  return (
    <div className="w-full h-full flex justify-center items-start bg-transparent select-none">
      <div
        data-testid="notch-sensor"
        onMouseEnter={hoverStart}
        onMouseLeave={hoverEnd}
        className="relative flex items-start justify-center"
      >
        {dark && <Ear side="left" hidden={idle} />}

        <div
          data-testid="notch-container"
          data-stage={stage}
          style={pillStyle(screen, stage)}
          onClick={() => stage !== "expanded" && open()}
          className={`notch-spring notch-surface relative overflow-hidden border-0 outline-none ${
            dark || idle ? "bg-[#000000] text-white" : "bg-[#f8fafc] text-slate-900"
          } ${SHAPE[stage]}`}
        >
          <PeekPill
            visible={stage === "peek"}
            needsSetup={telemetry.needsSetup}
            youtube={telemetry.youtube}
            layout={screen.layout}
            notchWidth={screen.metrics.notchWidth}
          />
          <Dashboard
            theme={theme}
            visible={stage === "expanded"}
            telemetry={telemetry}
            tab={tab}
            onTab={setTab}
            onClose={close}
            enabledMetrics={enabledMetrics}
            onToggleMetric={toggleMetric}
            onToggleTheme={() => setDark((current) => !current)}
          />
        </div>

        {dark && <Ear side="right" hidden={idle} />}
      </div>
    </div>
  );
}
