import { render, screen, waitFor, fireEvent, act } from "@testing-library/react";
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

const NOTCH_SCREEN = {
  metrics: { hasNotch: true, notchWidth: 185, notchHeight: 32, menuBarHeight: 37 },
  layout: {
    peekWidth: 341,
    peekHeight: 32,
    expandedInsetTop: 32,
    idleWidth: 185,
    idleHeight: 32,
    wings: true,
  },
};

function mockBackend(opts: { configured: boolean; screen?: unknown }) {
  invokeMock.mockImplementation(async (cmd: string) => {
    if (cmd === "get_screen_info") return opts.screen ?? null;
    if (cmd === "get_credential_status") {
      return { youtubeConfigured: opts.configured, youtubeChannelConfigured: opts.configured };
    }
    if (cmd === "refresh_telemetry") {
      return { youtube: null, errors: {}, lastUpdatedUnixMs: 0 };
    }
    return null;
  });
}

describe("screen-aware layout and first run", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listeners.clear();
  });

  it("hugs a hardware notch: idle matches the housing, peek splits into wings", async () => {
    mockBackend({ configured: true, screen: NOTCH_SCREEN });
    render(<App />);
    const container = screen.getByTestId("notch-container");

    await waitFor(() => {
      expect(container.style.width).toBe("185px");
      expect(container.style.height).toBe("32px");
    });

    act(() => listeners.get("notch://hover")?.({ payload: true }));
    await waitFor(() => {
      expect(container.getAttribute("data-stage")).toBe("peek");
      expect(container.style.width).toBe("341px");
    });
  });

  it("clears the camera housing when expanded on a notched screen", async () => {
    mockBackend({ configured: true, screen: NOTCH_SCREEN });
    render(<App />);
    const container = screen.getByTestId("notch-container");
    await waitFor(() => expect(container.style.width).toBe("185px"));

    fireEvent.click(container);
    await waitFor(() => {
      expect(container.getAttribute("data-stage")).toBe("expanded");
      expect(container.style.height).toBe("248px"); // 216 + 32 inset
      expect(container.style.paddingTop).toBe("40px"); // inset + 8
    });
  });

  it("opens the welcome card by itself when no credentials exist", async () => {
    mockBackend({ configured: false });
    render(<App />);
    const container = screen.getByTestId("notch-container");

    await waitFor(() => expect(container.getAttribute("data-stage")).toBe("expanded"));
    expect(screen.getByText("Connect your channel")).toBeTruthy();
    expect(invokeMock).toHaveBeenCalledWith("set_notch_expanded", { expanded: true });
  });

  it("does not auto-open once credentials are configured", async () => {
    mockBackend({ configured: true });
    render(<App />);
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("get_credential_status"));
    expect(screen.getByTestId("notch-container").getAttribute("data-stage")).toBe("idle");
  });

  it("opens from the tray menu event", async () => {
    mockBackend({ configured: true });
    render(<App />);
    await waitFor(() => expect(listeners.has("notch://open")).toBe(true));
    act(() => listeners.get("notch://open")?.({ payload: null }));
    await waitFor(() =>
      expect(screen.getByTestId("notch-container").getAttribute("data-stage")).toBe("expanded"),
    );
  });
});
