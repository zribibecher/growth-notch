import { render, screen, waitFor, fireEvent, act, within } from "@testing-library/react";
import { describe, it, expect, vi, beforeEach } from "vitest";
import App from "../App";

const invokeMock = vi.fn();
const listeners = new Map<string, (event: { payload: unknown }) => void>();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, handler: (event: { payload: unknown }) => void) => {
    listeners.set(name, handler);
    return () => listeners.delete(name);
  }),
}));

function snapshot(over: Record<string, unknown> = {}) {
  return {
    youtube: {
      channelId: "UCsampleChannelIdsampleCh",
      channelTitle: "Sample Channel",
      customUrl: "@samplechannel",
      avatarUrl: null,
      subscriberCount: 1000,
      videoCount: 12,
      viewCount: 48200,
      likeCount: 130,
      commentCount: 23,
      latestVideoTitle: "Latest upload",
      latestVideoThumbnail: null,
      latestVideoPublishedAt: new Date(Date.now() - 2 * 86_400_000).toISOString(),
      latestVideoViews: 1100,
      latestVideoLikes: 62,
      latestVideoComments: 4,
      etag: "x",
      fromEtagCache: false,
      quotaCostLastCall: 3,
      totalQuotaUsedToday: 3,
      etag304Hits: 0,
      ...over,
    },
    errors: {},
    lastUpdatedUnixMs: Date.now() - 120_000,
  };
}

function mockBackend(snapshots: Array<ReturnType<typeof snapshot>>, configured = true) {
  let call = 0;
  invokeMock.mockImplementation(async (cmd: string) => {
    if (cmd === "refresh_telemetry") {
      const snap = snapshots[Math.min(call, snapshots.length - 1)];
      call += 1;
      return snap;
    }
    if (cmd === "get_credential_status" || cmd === "save_api_key" || cmd === "delete_api_key") {
      return { youtubeConfigured: configured, youtubeChannelConfigured: configured };
    }
    return null;
  });
}

async function openDashboard() {
  const container = screen.getByTestId("notch-container");
  fireEvent.click(container);
  await waitFor(() => expect(container.getAttribute("data-stage")).toBe("expanded"));
  return container;
}

describe("dashboard flows", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listeners.clear();
  });

  it("shows how far subscribers moved since launch after a refresh", async () => {
    mockBackend([snapshot({ subscriberCount: 1000 }), snapshot({ subscriberCount: 1020 })]);
    render(<App />);
    await openDashboard();
    await waitFor(() => expect(screen.getByTestId("card-subscribers").textContent).toBe("1.0K"));
    expect(screen.queryByTestId("subs-delta")).toBeNull();

    fireEvent.click(screen.getByTestId("refresh-btn"));
    await waitFor(() => expect(screen.getByTestId("subs-delta").textContent).toBe("+20"));
  });

  it("labels the latest upload with its age", async () => {
    mockBackend([snapshot()]);
    render(<App />);
    await openDashboard();
    await waitFor(() => expect(screen.getByText(/Latest upload · 2d ago/i)).toBeTruthy());
  });

  it("renders a trusted thumbnail and ignores an untrusted one", async () => {
    mockBackend([snapshot({ latestVideoThumbnail: "https://i.ytimg.com/vi/abc/mqdefault.jpg" })]);
    const first = render(<App />);
    await openDashboard();
    await waitFor(() =>
      expect(document.querySelector("img")?.getAttribute("src")).toContain("i.ytimg.com"),
    );
    first.unmount();

    mockBackend([snapshot({ latestVideoThumbnail: "https://evil.example.com/pixel.png" })]);
    render(<App />);
    await openDashboard();
    await waitFor(() => expect(screen.getByTestId("card-subscribers")).toBeTruthy());
    expect(document.querySelector("img")).toBeNull();
  });

  it("turns extra metrics into chips and keeps the headline trio", async () => {
    mockBackend([snapshot()]);
    render(<App />);
    await openDashboard();
    fireEvent.click(screen.getByTestId("metrics-tab-btn"));
    fireEvent.click(screen.getByTestId("toggle-metric-views"));
    fireEvent.click(screen.getByTestId("pulse-tab-btn"));
    await waitFor(() => expect(screen.getByTestId("card-views").textContent).toBe("48.2K"));
    expect(screen.getByTestId("card-likes").textContent).toBe("130");
    expect(screen.getByTestId("card-comments").textContent).toBe("23");
  });

  it("closes the dashboard with Escape", async () => {
    mockBackend([snapshot()]);
    render(<App />);
    const container = await openDashboard();
    fireEvent.keyDown(window, { key: "Escape" });
    await waitFor(() => expect(container.getAttribute("data-stage")).toBe("idle"));
  });

  it("exposes tabs and labelled icon buttons to assistive tech", async () => {
    mockBackend([snapshot()]);
    render(<App />);
    await openDashboard();
    const tablist = screen.getByRole("tablist");
    const tabs = within(tablist).getAllByRole("tab");
    expect(tabs.map((t) => t.textContent)).toEqual(["Pulse", "Metrics", "Setup"]);
    expect(tabs[0].getAttribute("aria-selected")).toBe("true");
    expect(screen.getByLabelText("Close dashboard")).toBeTruthy();
    expect(screen.getByLabelText("Refresh now")).toBeTruthy();
    expect(screen.getByLabelText("Toggle light or dark theme")).toBeTruthy();
  });
});

describe("setup flow", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listeners.clear();
  });

  async function openSetup() {
    mockBackend([snapshot()]);
    render(<App />);
    await openDashboard();
    fireEvent.click(screen.getByTestId("setup-tab-btn"));
    return screen.getByLabelText("Secret Value") as HTMLInputElement;
  }

  it("saves a pasted channel ID as a channel ID", async () => {
    const input = await openSetup();
    fireEvent.change(input, { target: { value: "UCabcdefghijklmnopqrstuv" } });
    fireEvent.click(screen.getByTestId("save-secret-btn"));
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("save_api_key", {
        provider: "youtube_channel_id",
        apiKey: "UCabcdefghijklmnopqrstuv",
      }),
    );
    expect(input.value).toBe("");
  });

  it("refuses text it cannot classify and never calls the backend", async () => {
    const input = await openSetup();
    fireEvent.change(input, { target: { value: "@mychannel" } });
    fireEvent.click(screen.getByTestId("save-secret-btn"));
    const status = await screen.findByTestId("status-msg");
    expect(status.textContent).toContain("Not recognised");
    expect(invokeMock).not.toHaveBeenCalledWith("save_api_key", expect.anything());
  });

  it("clears both saved credentials", async () => {
    await openSetup();
    fireEvent.click(screen.getByTestId("delete-secret-btn"));
    await waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("delete_api_key", { provider: "youtube_api_key" });
      expect(invokeMock).toHaveBeenCalledWith("delete_api_key", { provider: "youtube_channel_id" });
    });
  });

  it("never echoes a pasted secret back into the page", async () => {
    const input = await openSetup();
    fireEvent.change(input, { target: { value: "AIzaSyDoNotShowThisKeyAnywhere12345" } });
    fireEvent.click(screen.getByTestId("save-secret-btn"));
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("save_api_key", expect.anything()));
    expect(document.body.innerHTML).not.toContain("DoNotShowThisKey");
    expect(window.localStorage.length).toBe(0);
    expect(window.sessionStorage.length).toBe(0);
  });

  it("survives a malformed layout event", async () => {
    await openSetup();
    act(() => listeners.get("notch://layout")?.({ payload: { nonsense: true } }));
    expect(screen.getByTestId("notch-container")).toBeTruthy();
  });
});

describe("keyboard focus", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listeners.clear();
  });

  it("asks for keyboard focus only while the Setup tab is showing", async () => {
    mockBackend([snapshot()]);
    render(<App />);
    const container = await openDashboard();

    // Pulse tab: the panel must not take typing from the app underneath.
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("set_keyboard_focus", { allow: false }),
    );
    expect(invokeMock).not.toHaveBeenCalledWith("set_keyboard_focus", { allow: true });

    fireEvent.click(screen.getByTestId("setup-tab-btn"));
    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("set_keyboard_focus", { allow: true }),
    );

    fireEvent.click(screen.getByTestId("pulse-tab-btn"));
    await waitFor(() => {
      const calls = invokeMock.mock.calls.filter((c) => c[0] === "set_keyboard_focus");
      expect(calls[calls.length - 1][1]).toEqual({ allow: false });
    });
    expect(container.getAttribute("data-stage")).toBe("expanded");
  });
});

describe("window size follows the dashboard", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listeners.clear();
  });

  it("still shrinks the native window when hover events arrive during the retract", async () => {
    mockBackend([snapshot()]);
    render(<App />);
    const container = await openDashboard();

    fireEvent.click(screen.getByTestId("collapse-btn"));
    await waitFor(() => expect(container.getAttribute("data-stage")).toBe("idle"));

    // The cursor is still near the pill, so hover events keep coming while it retracts.
    act(() => listeners.get("notch://hover")?.({ payload: true }));
    act(() => listeners.get("notch://hover")?.({ payload: false }));
    act(() => listeners.get("notch://hover")?.({ payload: true }));
    act(() => listeners.get("notch://hover")?.({ payload: false }));

    await waitFor(
      () => expect(invokeMock).toHaveBeenCalledWith("set_notch_expanded", { expanded: false }),
      { timeout: 2000 },
    );
  });

  it("grows the window again if the dashboard reopens before the shrink fires", async () => {
    mockBackend([snapshot()]);
    render(<App />);
    const container = await openDashboard();

    fireEvent.click(screen.getByTestId("collapse-btn"));
    await waitFor(() => expect(container.getAttribute("data-stage")).toBe("idle"));
    fireEvent.click(container);
    await waitFor(() => expect(container.getAttribute("data-stage")).toBe("expanded"));

    // Wait past the shrink delay: the cancelled shrink must not fire while the dashboard is open.
    await new Promise((r) => setTimeout(r, 800));
    const shrinkCalls = invokeMock.mock.calls.filter(
      (c) => c[0] === "set_notch_expanded" && (c[1] as { expanded: boolean }).expanded === false,
    );
    expect(shrinkCalls.length).toBe(0);
  });
});

describe("messages and onboarding", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listeners.clear();
  });

  it("shows a plain-language error on Pulse and keeps the last numbers", async () => {
    const withError = {
      ...snapshot(),
      errors: { youtube: "YouTube rejected the API key. Paste it again in Setup." },
    };
    mockBackend([withError]);
    render(<App />);
    await openDashboard();

    const status = await screen.findByTestId("status-msg");
    expect(status.textContent).toBe("YouTube rejected the API key. Paste it again in Setup.");
    expect(screen.getByTestId("card-subscribers").textContent).toBe("1.0K");
  });

  it("repeats the same error on the Setup tab, where it can be fixed", async () => {
    const withError = {
      ...snapshot(),
      errors: { youtube: "Daily quota used up. It resets at midnight Pacific." },
    };
    mockBackend([withError]);
    render(<App />);
    await openDashboard();
    fireEvent.click(screen.getByTestId("setup-tab-btn"));

    const status = await screen.findByTestId("status-msg");
    expect(status.textContent).toContain("Daily quota used up");
  });

  it("walks a new user from the welcome card to the Setup field", async () => {
    mockBackend([snapshot()], false);
    render(<App />);
    const container = screen.getByTestId("notch-container");
    await waitFor(() => expect(container.getAttribute("data-stage")).toBe("expanded"));

    fireEvent.click(screen.getByTestId("start-setup-btn"));
    expect(screen.getByLabelText("Secret Value")).toBeTruthy();
    expect(screen.getByText("API key missing")).toBeTruthy();
    expect(screen.getByText("Channel ID missing")).toBeTruthy();
  });

  it("lists every metric as a switch with its full label", async () => {
    mockBackend([snapshot()]);
    render(<App />);
    await openDashboard();
    fireEvent.click(screen.getByTestId("metrics-tab-btn"));

    const switches = screen.getAllByRole("switch");
    expect(switches).toHaveLength(13);
    expect(screen.getByTestId("toggle-metric-latest_comments").textContent).toContain(
      "Latest Comments",
    );
    expect(screen.getByTestId("toggle-metric-subscribers").getAttribute("aria-checked")).toBe(
      "true",
    );
    expect(screen.getByTestId("toggle-metric-views").getAttribute("aria-checked")).toBe("false");

    fireEvent.click(screen.getByTestId("toggle-metric-views"));
    expect(screen.getByTestId("toggle-metric-views").getAttribute("aria-checked")).toBe("true");
  });

  it("falls back to subscribers when every metric is switched off", async () => {
    mockBackend([snapshot()]);
    render(<App />);
    await openDashboard();
    fireEvent.click(screen.getByTestId("metrics-tab-btn"));
    for (const key of ["subscribers", "likes", "comments", "latest_upload_bar"]) {
      fireEvent.click(screen.getByTestId(`toggle-metric-${key}`));
    }
    fireEvent.click(screen.getByTestId("pulse-tab-btn"));
    await waitFor(() => expect(screen.getByTestId("card-subscribers").textContent).toBe("1.0K"));
    expect(screen.queryByTestId("latest-video-title")).toBeNull();
  });

  it("switches to the light theme and back", async () => {
    mockBackend([snapshot()]);
    render(<App />);
    const container = await openDashboard();
    expect(container.className).toContain("bg-[#000000]");

    fireEvent.click(screen.getByTestId("theme-toggle-btn"));
    expect(container.className).toContain("bg-[#f8fafc]");
    fireEvent.click(screen.getByTestId("theme-toggle-btn"));
    expect(container.className).toContain("bg-[#000000]");
  });
});
