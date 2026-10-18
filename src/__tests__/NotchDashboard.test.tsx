import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import App from "../App";

const invokeMock = vi.fn();
let hoverEventCallback: ((event: { payload: boolean }) => void) | null = null;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (eventName: string, handler: (event: { payload: boolean }) => void) => {
    if (eventName === "notch://hover") {
      hoverEventCallback = handler;
    }
    return () => {
      hoverEventCallback = null;
    };
  }),
}));

describe("YouTube Growth Notch Widget & Security", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    hoverEventCallback = null;
    window.localStorage.clear();
    window.sessionStorage.clear();

    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "refresh_telemetry") {
        return {
          youtube: {
            channelId: "UC_TECH_UNTANGLED",
            channelTitle: "Tech Untangled",
            customUrl: "@techuntangled",
            avatarUrl: null,
            subscriberCount: 142500,
            videoCount: 94,
            viewCount: 8910200,
            likeCount: 34200,
            commentCount: 4850,
            latestVideoTitle: "Building a Local Rust Notch Widget",
            latestVideoViews: 52300,
            latestVideoLikes: 4100,
            latestVideoComments: 380,
            etag: '"etag-yt-v1"',
            fromEtagCache: true,
            quotaCostLastCall: 0,
            totalQuotaUsedToday: 1,
            etag304Hits: 5,
          },
          errors: {},
          lastUpdatedUnixMs: Date.now(),
        };
      }
      if (cmd === "get_credential_status" || cmd === "save_api_key" || cmd === "delete_api_key") {
        return {
          youtubeConfigured: true,
          youtubeChannelConfigured: true,
        };
      }
      return null;
    });
  });

  it("stays invisible when idle, reveals compact pill on hover (peek) with auto-refresh, and opens full dashboard only on click (expanded)", async () => {
    render(<App />);

    const notchSensor = screen.getByTestId("notch-sensor");
    const notchContainer = screen.getByTestId("notch-container");
    const peekLayer = screen.getByTestId("peek-pill-layer");

    // 1. Starts idle: a black pill the size of the hover target, with its text hidden
    expect(notchContainer.getAttribute("data-stage")).toBe("idle");
    expect(notchContainer.className).toContain("w-[132px]");
    expect(notchContainer.className).toContain("h-[24px]");
    // Idle is invisible: nothing is drawn on the menu bar until the cursor arrives
    expect(notchContainer.className).toContain("opacity-0");
    expect(notchContainer.className).toContain("bg-[#000000]");
    expect(notchContainer.className).toContain("border-0");
    expect(peekLayer.className).toContain("opacity-0");

    await waitFor(() => {
      expect(screen.getByTestId("compact-subs").textContent).toBe("142.5K");
      expect(screen.getByTestId("compact-likes").textContent).toBe("34.2K");
      expect(screen.getByTestId("compact-comments").textContent).toBe("4.8K");
    });

    const refreshCallsBeforeHover = invokeMock.mock.calls.filter(
      (c) => c[0] === "refresh_telemetry",
    ).length;

    // 2. Hovering reveals the compact pill ("peek") and auto-refreshes WITHOUT opening the full window
    fireEvent.mouseEnter(notchSensor);

    await waitFor(() => {
      expect(notchContainer.getAttribute("data-stage")).toBe("peek");
      expect(notchContainer.className).toContain("w-[248px]");
      expect(peekLayer.className).toContain("opacity-100");
      const refreshCallsAfterHover = invokeMock.mock.calls.filter(
        (c) => c[0] === "refresh_telemetry",
      ).length;
      expect(refreshCallsAfterHover).toBeGreaterThan(refreshCallsBeforeHover);
    });
    expect(invokeMock).not.toHaveBeenCalledWith("set_notch_expanded", { expanded: true });

    // 3. Clicking the hovered pill opens the full expanded studio dashboard
    fireEvent.click(notchContainer);

    await waitFor(() => {
      expect(notchContainer.getAttribute("data-stage")).toBe("expanded");
      expect(invokeMock).toHaveBeenCalledWith("set_notch_expanded", { expanded: true });
      expect(screen.getByTestId("card-subscribers").textContent).toBe("142.5K");
      expect(screen.getByTestId("card-likes").textContent).toBe("34.2K");
      expect(screen.getByTestId("card-comments").textContent).toBe("4.8K");
      expect(screen.queryByTestId("card-views")).toBeNull();
    });

    // Open + Metrics menu and toggle Total Views + Videos ON
    fireEvent.click(screen.getByTestId("metrics-tab-btn"));
    fireEvent.click(screen.getByTestId("toggle-metric-views"));
    fireEvent.click(screen.getByTestId("toggle-metric-videos"));

    // Return to Pulse tab and verify newly enabled metrics appear
    fireEvent.click(screen.getByTestId("pulse-tab-btn"));
    await waitFor(() => {
      expect(screen.getByTestId("card-views").textContent).toBe("8.9M");
      expect(screen.getByTestId("card-videos").textContent).toBe("94");
      expect(screen.getByTestId("latest-video-title").textContent).toBe(
        "Building a Local Rust Notch Widget",
      );
    });

    // Toggle Light / Dark theme
    expect(notchContainer.className).toContain("bg-[#000000]");
    fireEvent.click(screen.getByTestId("theme-toggle-btn"));
    expect(notchContainer.className).toContain("bg-[#f8fafc]");

    // Ensure removed clutter (Instagram, TikTok, Quota, 304 ETag, footer text) is absent
    const html = document.body.innerHTML;
    expect(html).not.toContain("Instagram");
    expect(html).not.toContain("TikTok");
    expect(html).not.toContain("Quota:");
    expect(html).not.toContain("304 ETag");
    expect(html).not.toContain("Zero-Cost Local BYOK");

    // 4. Moving the mouse out retracts back to idle
    fireEvent.mouseLeave(notchSensor);

    await waitFor(() => {
      expect(notchContainer.getAttribute("data-stage")).toBe("idle");
      expect(invokeMock).toHaveBeenCalledWith("set_notch_expanded", { expanded: false });
    });
  });

  it("sends secrets only to save_api_key and never touches browser storage", async () => {
    render(<App />);

    fireEvent.click(screen.getByTestId("notch-container"));
    fireEvent.click(await screen.findByTestId("setup-tab-btn"));

    // Empty secret submission must be rejected client-side
    fireEvent.click(screen.getByTestId("save-secret-btn"));
    const statusNode = await screen.findByTestId("status-msg");
    expect(statusNode.textContent).toContain("Credential value cannot be empty");

    const secretInput = screen.getByLabelText("Secret Value") as HTMLInputElement;
    fireEvent.change(secretInput, { target: { value: "AIzaSySecretKeyNeverInLocalStorage" } });
    fireEvent.click(screen.getByTestId("save-secret-btn"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("save_api_key", {
        provider: "youtube_api_key",
        apiKey: "AIzaSySecretKeyNeverInLocalStorage",
      });
    });

    expect(secretInput.value).toBe("");
    expect(window.localStorage.length).toBe(0);
    expect(window.sessionStorage.length).toBe(0);
    expect(document.body.innerHTML).not.toContain("AIzaSySecretKeyNeverInLocalStorage");
  });

  it("unfolds on the very first unfocused hover via native macOS 'notch://hover' event listener and retracts on false", async () => {
    render(<App />);

    const notchContainer = screen.getByTestId("notch-container");
    expect(notchContainer.getAttribute("data-stage")).toBe("idle");
    expect(hoverEventCallback).not.toBeNull();

    // Simulate native NSEvent::mouseLocation hover event when window is unfocused
    act(() => {
      hoverEventCallback?.({ payload: true });
    });

    await waitFor(() => {
      expect(notchContainer.getAttribute("data-stage")).toBe("peek");
    });

    // Simulate native cursor leaving the Notch frame
    act(() => {
      hoverEventCallback?.({ payload: false });
    });

    await waitFor(() => {
      expect(notchContainer.getAttribute("data-stage")).toBe("idle");
    });
  });

  it("stays open on the Setup tab when the cursor leaves, so typing is never interrupted", async () => {
    render(<App />);

    const notchSensor = screen.getByTestId("notch-sensor");
    const notchContainer = screen.getByTestId("notch-container");

    // Expand and switch to the Setup tab
    fireEvent.click(notchContainer);
    fireEvent.click(await screen.findByTestId("setup-tab-btn"));
    expect(notchContainer.getAttribute("data-stage")).toBe("expanded");

    // Leaving while on the Setup tab must not close the dashboard
    fireEvent.mouseLeave(notchSensor);
    await new Promise((r) => setTimeout(r, 130));
    expect(notchContainer.getAttribute("data-stage")).toBe("expanded");
  });

  it("throttles hover auto-refresh with a 15-second cooldown against rapid cursor brushes", async () => {
    render(<App />);

    const notchSensor = screen.getByTestId("notch-sensor");
    await waitFor(() => {
      expect(screen.getByTestId("compact-subs").textContent).toBe("142.5K");
    });

    // First hover triggers immediate refresh
    fireEvent.mouseEnter(notchSensor);
    await waitFor(() => {
      expect(screen.getByTestId("notch-container").getAttribute("data-stage")).toBe("peek");
    });

    const countAfterFirstHover = invokeMock.mock.calls.filter(
      (c) => c[0] === "refresh_telemetry",
    ).length;

    // Immediate second hover within 15s cooldown must NOT spam refresh_telemetry
    fireEvent.mouseLeave(notchSensor);
    fireEvent.mouseEnter(notchSensor);

    const countAfterSecondHover = invokeMock.mock.calls.filter(
      (c) => c[0] === "refresh_telemetry",
    ).length;
    expect(countAfterSecondHover).toBe(countAfterFirstHover);
  });

  it("blocks untrusted or HTTP avatar URLs in the UI while rendering trusted HTTPS YouTube CDN avatars", async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "refresh_telemetry") {
        return {
          youtube: {
            channelId: "UC_TECH_UNTANGLED",
            channelTitle: "Tech Untangled",
            customUrl: "@techuntangled",
            avatarUrl: "https://evil.example.com/tracker.png",
            subscriberCount: 142500,
            videoCount: 94,
            viewCount: 8910200,
            likeCount: 34200,
            commentCount: 4850,
            latestVideoTitle: null,
            latestVideoViews: 0,
            latestVideoLikes: 0,
            latestVideoComments: 0,
            etag: '"etag-1"',
            fromEtagCache: false,
            quotaCostLastCall: 1,
            totalQuotaUsedToday: 1,
            etag304Hits: 0,
          },
          errors: {},
          lastUpdatedUnixMs: Date.now(),
        };
      }
      return { youtubeConfigured: true, youtubeChannelConfigured: true };
    });

    render(<App />);
    fireEvent.click(screen.getByTestId("notch-container"));

    await waitFor(() => {
      expect(screen.getByTestId("notch-container").getAttribute("data-stage")).toBe("expanded");
    });

    // Untrusted avatar domain must never render an <img> tag
    expect(document.querySelector("img")).toBeNull();
  });

  it("deletes saved credentials from the Setup tab", async () => {
    render(<App />);

    fireEvent.click(screen.getByTestId("notch-container"));
    fireEvent.click(await screen.findByTestId("setup-tab-btn"));
    fireEvent.click(screen.getByTestId("delete-secret-btn"));

    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("delete_api_key", {
        provider: "youtube_api_key",
      });
    });
  });
});
