const TRUSTED_IMAGE_HOSTS = new Set(["yt3.ggpht.com", "yt3.googleusercontent.com", "i.ytimg.com"]);

/**
 * True for plain-HTTPS URLs on YouTube's image hosts. The Rust side checks this too and the
 * content security policy enforces it a third time; this keeps a bad URL from even reaching an <img>.
 */
export function isTrustedImageUrl(url: string | null | undefined): url is string {
  if (!url || url.includes("..") || url.includes("\\")) return false;
  try {
    const parsed = new URL(url);
    return (
      parsed.protocol === "https:" &&
      !parsed.username &&
      !parsed.password &&
      !parsed.port &&
      TRUSTED_IMAGE_HOSTS.has(parsed.hostname)
    );
  } catch {
    return false;
  }
}
