export interface YouTubeMetrics {
  channelId: string;
  channelTitle: string | null;
  customUrl: string | null;
  avatarUrl: string | null;
  country?: string | null;
  publishedAt?: string | null;
  subscriberCount: number;
  videoCount: number;
  viewCount: number;
  likeCount: number;
  commentCount: number;
  latestVideoTitle: string | null;
  latestVideoThumbnail?: string | null;
  latestVideoPublishedAt?: string | null;
  latestVideoViews: number;
  latestVideoLikes: number;
  latestVideoComments: number;
  etag: string;
  fromEtagCache: boolean;
  quotaCostLastCall: number;
  totalQuotaUsedToday: number;
  etag304Hits: number;
}

export interface TelemetrySnapshot {
  youtube: YouTubeMetrics | null;
  errors: Record<string, string>;
  lastUpdatedUnixMs: number;
}

export interface CredentialStatus {
  youtubeConfigured: boolean;
  youtubeChannelConfigured: boolean;
}

export interface LayoutHints {
  peekWidth: number;
  peekHeight: number;
  expandedInsetTop: number;
  idleWidth: number;
  idleHeight: number;
  wings: boolean;
}

export interface ScreenMetrics {
  hasNotch: boolean;
  notchWidth: number;
  notchHeight: number;
  menuBarHeight: number;
}

export interface ScreenInfo {
  metrics: ScreenMetrics;
  layout: LayoutHints;
}

export type MetricKey =
  | "subscribers"
  | "likes"
  | "comments"
  | "views"
  | "videos"
  | "avg_views"
  | "engagement_rate"
  | "latest_views"
  | "latest_likes"
  | "latest_comments"
  | "country"
  | "joined_year"
  | "latest_upload_bar";
