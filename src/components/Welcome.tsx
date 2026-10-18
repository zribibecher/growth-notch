import { PRIMARY_BUTTON, type Theme } from "../lib/theme";

/** First-run card on the Pulse tab, shown until a key and channel ID are saved. */
export function Welcome({ theme: t, onStart }: { theme: Theme; onStart: () => void }) {
  return (
    <div className="flex flex-col items-center justify-center text-center gap-2 flex-1">
      <div className={`text-[15px] font-semibold ${t.strong}`}>Connect your channel</div>
      <div className={`text-[11px] leading-relaxed max-w-[300px] ${t.muted}`}>
        Free and local. Paste a YouTube Data API key and your channel ID, and your numbers appear
        here. Nothing leaves your Mac except requests to YouTube.
      </div>
      <button
        type="button"
        data-testid="start-setup-btn"
        onClick={(e) => {
          e.stopPropagation();
          onStart();
        }}
        className={`mt-1 px-4 py-1.5 rounded-full text-[11.5px] ${PRIMARY_BUTTON}`}
      >
        Set up in 30 seconds
      </button>
    </div>
  );
}
