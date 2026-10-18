export type CredentialKind = "youtube_api_key" | "youtube_channel_id";

/** Tells a pasted API key from a channel ID, so Setup needs one field instead of a dropdown. */
export function classifyCredential(raw: string): CredentialKind | null {
  const value = raw.trim();
  if (/^AIza/.test(value)) return "youtube_api_key";
  if (/^UC[\w-]{20,}$/.test(value)) return "youtube_channel_id";
  return null;
}
