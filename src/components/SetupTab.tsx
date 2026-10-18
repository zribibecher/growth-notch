import { useState, type FormEvent } from "react";
import { classifyCredential, type CredentialKind } from "../lib/credentials";
import { PRIMARY_BUTTON, type Theme } from "../lib/theme";
import type { CredentialStatus } from "../types/telemetry";

interface Props {
  theme: Theme;
  credentials: CredentialStatus;
  /** The last refresh or save error, if any. */
  error: string | null;
  onSave: (kind: CredentialKind, value: string) => void;
  onClear: () => void;
}

/** One field: whatever gets pasted is recognised as an API key or a channel ID. */
export function SetupTab({ theme: t, credentials, error, onSave, onClear }: Props) {
  const [value, setValue] = useState("");
  const [problem, setProblem] = useState<string | null>(null);

  const submit = (event: FormEvent) => {
    event.preventDefault();
    const pasted = value.trim();
    if (!pasted) return setProblem("Credential value cannot be empty");
    const kind = classifyCredential(pasted);
    if (!kind) {
      return setProblem("Not recognised. Paste an API key (AIza…) or channel ID (UC…).");
    }
    setProblem(null);
    setValue("");
    onSave(kind, pasted);
  };

  const message = problem ?? error;

  return (
    <form
      onClick={(e) => e.stopPropagation()}
      onSubmit={submit}
      className="flex flex-col gap-2.5 flex-1 min-h-0"
    >
      <div>
        <div className={`text-[13px] font-semibold ${t.strong}`}>Connect your channel</div>
        <div className={`text-[10.5px] leading-snug ${t.muted}`}>
          Paste your API key, then your channel ID. We detect which is which.
        </div>
      </div>

      <div className="flex gap-2">
        <input
          type="password"
          aria-label="Secret Value"
          autoComplete="off"
          spellCheck={false}
          placeholder="AIza… API key  or  UC… channel ID"
          value={value}
          onChange={(e) => {
            setValue(e.target.value);
            setProblem(null);
          }}
          className={`${t.input} flex-1 min-w-0`}
        />
        <button
          type="submit"
          data-testid="save-secret-btn"
          className={`px-4 rounded-lg text-[12px] ${PRIMARY_BUTTON}`}
        >
          Save
        </button>
      </div>

      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-1.5">
          <span className={t.statusChip(credentials.youtubeConfigured)}>
            API key {credentials.youtubeConfigured ? "✓" : "missing"}
          </span>
          <span className={t.statusChip(credentials.youtubeChannelConfigured)}>
            Channel ID {credentials.youtubeChannelConfigured ? "✓" : "missing"}
          </span>
        </div>
        <button
          type="button"
          data-testid="delete-secret-btn"
          onClick={onClear}
          className={`text-[10.5px] font-medium underline-offset-2 hover:underline ${t.muted}`}
        >
          Clear saved
        </button>
      </div>

      {/* One slot for guidance, so an error replaces the help instead of pushing it off the card. */}
      <div className="mt-auto text-[10px] leading-snug">
        {message ? (
          <span data-testid="status-msg" role="status" aria-live="polite" className={t.warning}>
            {message}
          </span>
        ) : (
          <span className={t.muted}>
            Free key: enable YouTube Data API v3 in Google Cloud Console, then create an API key.
            Channel ID: YouTube Studio, Settings, Channel, Advanced.
          </span>
        )}
      </div>
    </form>
  );
}
