import { useState, type ReactNode } from "react";
import { isTrustedImageUrl } from "../lib/images";

interface Props {
  src: string | null | undefined;
  /** Changes on every refresh, so an image that failed to load gets another try. */
  version: number;
  className: string;
  /** Shown when there is no image, it is not on a YouTube host, or it failed to load. */
  fallback?: ReactNode;
}

/** An image from YouTube's CDN, shown only if the URL is trusted. */
export function RemoteImage({ src, version, className, fallback = null }: Props) {
  const attempt = `${src}|${version}`;
  const [failedAttempt, setFailedAttempt] = useState<string | null>(null);

  if (!isTrustedImageUrl(src) || failedAttempt === attempt) return <>{fallback}</>;
  return (
    <img
      src={src}
      alt=""
      referrerPolicy="no-referrer"
      crossOrigin="anonymous"
      onError={() => setFailedAttempt(attempt)}
      className={className}
    />
  );
}
