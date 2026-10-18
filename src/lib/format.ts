/** 842 -> "842", 142500 -> "142.5K", 8910200 -> "8.9M". */
export function formatCompactNumber(value: number | undefined | null): string {
  if (value === undefined || value === null) return "—";
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}K`;
  return value.toString();
}

/** "just now", "42s ago", "5m ago", "3h ago", "2d ago". */
export function timeAgo(thenMs: number, nowMs: number): string {
  if (!Number.isFinite(thenMs)) return "";
  const seconds = Math.max(0, Math.round((nowMs - thenMs) / 1000));
  if (seconds < 10) return "just now";
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.round(hours / 24)}d ago`;
}
