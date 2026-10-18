import type { MetricDefinition } from "./metrics";

export const FOCUS_RING =
  "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-red-500/70";

/** The red call-to-action look, shared by the welcome button and Save. */
export const PRIMARY_BUTTON =
  "bg-gradient-to-r from-red-600 to-rose-600 hover:from-red-500 hover:to-rose-500 font-semibold text-white transition-all";

/** Every colour decision for the dashboard, so components don't repeat dark/light ternaries. */
export interface Theme {
  dark: boolean;
  strong: string;
  muted: string;
  /** Colour for error and status messages. */
  warning: string;
  hairline: string;
  glass: string;
  input: string;
  iconButton: string;
  tab: (active: boolean) => string;
  statusChip: (ok: boolean) => string;
  metricToggle: (active: boolean) => string;
  tone: (metric: MetricDefinition) => string;
}

export function makeTheme(dark: boolean): Theme {
  const pick = (onDark: string, onLight: string) => (dark ? onDark : onLight);
  const strong = pick("text-white", "text-slate-900");

  return {
    dark,
    strong,
    muted: pick("text-white/45", "text-slate-500"),
    warning: pick("text-amber-400", "text-amber-700"),
    hairline: pick("border-white/[0.08]", "border-slate-200"),
    glass: pick("bg-white/[0.045] border-white/[0.07]", "bg-white border-slate-200/90"),
    input: `rounded-lg px-3 py-2 text-[12px] border focus:outline-none focus:ring-1 ${pick(
      "bg-white/[0.05] border-white/[0.12] text-white placeholder:text-white/30 focus:ring-red-500/60",
      "bg-white border-slate-300 text-slate-900 focus:ring-red-400",
    )}`,
    iconButton: `w-6 h-6 rounded-full flex items-center justify-center transition-colors ${FOCUS_RING} ${pick(
      "text-white/50 hover:text-white hover:bg-white/10",
      "text-slate-500 hover:text-slate-900 hover:bg-slate-200",
    )}`,
    tab: (active) =>
      `px-2.5 py-1 rounded-full text-[11px] font-medium transition-colors ${FOCUS_RING} ${
        active
          ? pick("bg-white/10 text-white", "bg-slate-900/[0.07] text-slate-900")
          : pick("text-white/40 hover:text-white/80", "text-slate-500 hover:text-slate-800")
      }`,
    statusChip: (ok) =>
      `px-2 py-0.5 rounded-full border text-[10px] font-medium ${
        ok
          ? pick(
              "bg-emerald-500/15 border-emerald-500/30 text-emerald-300",
              "bg-emerald-50 border-emerald-300 text-emerald-700",
            )
          : pick(
              "bg-white/5 border-white/10 text-white/45",
              "bg-slate-100 border-slate-300 text-slate-500",
            )
      }`,
    metricToggle: (active) =>
      `px-2 py-1.5 rounded-lg text-[10px] font-medium border text-left flex items-center justify-between gap-1 transition-all ${
        active
          ? pick(
              "bg-red-500/15 border-red-500/40 text-white",
              "bg-red-50 border-red-300 text-red-900",
            )
          : pick(
              "bg-white/[0.03] border-white/[0.07] text-white/50 hover:text-white",
              "bg-white border-slate-200 text-slate-600 hover:text-slate-900",
            )
      }`,
    tone: (metric) => (metric.tone ? pick(metric.tone.dark, metric.tone.light) : strong),
  };
}
