# Architecture

Growth Notch is a Tauri 2 app. The Rust side owns the network, the secrets and the native window. A small React front end draws the pill. They talk through Tauri commands and events.

```
 YouTube Data API v3 <--HTTPS (rustls)-- Rust ------IPC------ React
                                          telemetry.rs        App.tsx, hooks/, components/
                                          credentials.rs
                                          window.rs, panel.rs, watcher.rs, tray.rs
```

## Modules

Rust, in `src-tauri/src/`:

| File | Responsibility |
|---|---|
| `lib.rs` | Starts the app: reads the screen, opens the vault, creates the window and tray, starts the cursor watcher. |
| `commands.rs` | The seven commands the web view may call. |
| `telemetry.rs` | YouTube client: `channels.list`, `playlistItems.list`, `videos.list`. ETag cache, a local quota estimate that resets at midnight Pacific, and error messages written for people. |
| `credentials.rs` | The encrypted vault for the API key and channel ID. |
| `security.rs` | Input validation, secret scrubbing, and the image URL allowlist. |
| `screen.rs` | Reads the target screen through AppKit (notch size, menu bar height). A pure `layout_for()` turns that into the pill layout. |
| `notch.rs` | Pure geometry: window bounds, the hover hit test, and the dwell rule. |
| `state.rs` | State shared by the commands, the watcher and the window code. |
| `window.rs` | Builds the window from `tauri.conf.json`, moves it, and follows screen changes. |
| `panel.rs` | The AppKit tweaks, macOS only: the `NSPanel` swap, window level and Spaces, keyboard focus gating, the haptic tick. |
| `watcher.rs` | Polls the cursor and tells the UI when to unfold, macOS only. |
| `tray.rs` | The menu bar icon. |

Web, in `src/`:

| Path | Responsibility |
|---|---|
| `App.tsx` | Puts the hooks and the two layers together. |
| `hooks/useNotch.ts` | The three stages (idle, peek, expanded) and the size of the native window behind them. |
| `hooks/useTelemetry.ts` | Refreshing, polling, saved-credential status and the subscriber change since launch. |
| `hooks/useScreenInfo.ts`, `hooks/useNativeEvent.ts` | Screen layout from the native side, and a safe way to listen to native events. |
| `components/` | `PeekPill`, `Dashboard` with its tabs, and small pieces such as `Icon` and `RemoteImage`. |
| `lib/` | Plain helpers: the metric catalog, theme classes, number and time formatting, the image URL check, screen layout. |

## The window

Growth Notch uses one transparent, borderless window that stays on top. Its flags live in the `notch` entry of `tauri.conf.json`, and `window.rs` builds it from there. It is centered on the top edge of the notched display, or on the primary display when there is no notch.

Panel. After Tauri creates the window, `panel.rs` swaps its class to an `NSPanel` subclass (`GrowthNotchPanel`) and sets the non-activating style bit. macOS only draws non-activating panels over another app's full-screen Space. Two checks guard the swap: the new class must not need more memory than the existing window (a unit test pins that `NSPanel` adds no instance variables), and `isKindOfClass:` must confirm it afterwards. If either fails, the app logs it and carries on as a plain window that cannot draw over full-screen apps. This is the riskiest `unsafe` code in the project.

Level and Spaces. The window sits at the status-bar level, joins all Spaces, stays put on Space changes and skips the window cycle.

Activation policy. The app runs as `Accessory`, so it has no Dock icon and never takes focus. `src-tauri/Info.plist` sets `LSUIElement` as well, because setting the policy in code only takes effect about a second after launch, and macOS would show a Dock icon until then. The tray icon is how you reopen it or quit.

Click-through. The window ignores mouse events everywhere except over the visible pill. The cursor watcher turns events on while the cursor is on the pill and off again the moment it leaves, so the window's transparent margins never catch clicks or scrolling meant for the page underneath.

Hover detection. `watcher.rs` reads `NSEvent.mouseLocation` and compares it with the pill rectangle for the current stage (idle, peek or expanded), computed from the window frame the app last set. That frame lives in shared state, so no AppKit window is touched from a background thread. Opening from idle needs the cursor to rest on the pill for about 125 ms, so a cursor that only crosses the top edge does nothing. Once the pill is open, being inside it is enough. This works while the app is unfocused, which WebKit hover events do not. The task polls every 25 ms near the window and every 100 ms when the cursor is far away. Set `GROWTH_NOTCH_DEBUG=1` to print every hover change to the terminal.

Keyboard focus. The panel's `canBecomeKeyWindow` is false by default. The UI turns it on only while the Setup tab is open, because that is the only place that needs typing, and the panel hands focus back as soon as it closes. This keeps the panel from taking keystrokes away from the app you are working in.

Screen changes. Every 1.5 seconds the screen is read again on the main thread. If the notch or frame changed, the window moves and the UI receives `notch://layout`.

Notch layout. `NSScreen.safeAreaInsets` plus `auxiliaryTopLeftArea` and `auxiliaryTopRightArea` give the size of the camera housing. With a notch, the pill is as tall as the housing and the stats sit in wings on either side. The dashboard leaves the housing height empty at the top. `NOTCH_SIM=1` simulates this on a Mac without a notch.

## Data flow

1. The UI calls `refresh_telemetry` on launch, every 60 seconds, and on hover (at most every 15 seconds).
2. Rust reads the credentials from the vault and calls `channels.list` with `If-None-Match`. When the channel changed, it also calls `playlistItems.list` and `videos.list` for the last 10 uploads. An unchanged channel answers `304` and the two upload calls are skipped.
3. The result is a `TelemetrySnapshot` that goes back to the UI. If a refresh fails, the previous numbers stay on screen next to a short message such as "YouTube rejected the API key. Paste it again in Setup."

Likes and comments are sums over the last 10 uploads. Subscriber counts are rounded by YouTube.

## Credentials

- `vault.enc` is a JSON map (`youtube_api_key`, `youtube_channel_id`) encrypted with ChaCha20-Poly1305, with a fresh random nonce on every write. It is written to a temporary file and renamed into place, so a crash never leaves a half-written vault.
- `.vault.key` is 32 random bytes in the same directory. Anyone who can read both files can decrypt the vault.
- The directory is mode `0700` (an older, looser directory is tightened) and both files are `0600`.
- A missing key, a truncated file or a tampered file opens as an empty vault. The app then asks for the credentials again.
- The macOS Keychain is not used, so there are no Keychain permission prompts. The cost is the one stated above: the key sits beside the vault.
- The UI never receives a secret back. It only gets booleans (`CredentialStatus`).

## Network and web view hardening

- The Content Security Policy is `default-src 'self'; script-src 'self'; object-src 'none'; base-uri 'self'; form-action 'none'; frame-ancestors 'none'; style-src 'self'; connect-src 'self' ipc: http://ipc.localhost`, plus `img-src` for the YouTube image hosts.
- The web view makes no API calls. They all happen in Rust. The web view only loads your avatar and thumbnails from the YouTube image hosts, without cookies or a referrer.
- Tauri permissions are explicit. The capability grants event listening and the seven commands, nothing else. `build.rs` declares those commands in an app manifest, so there are no default permissions, and a test checks that `build.rs`, the handler list in `lib.rs` and the capability agree.
- The API key is sent in the `x-goog-api-key` header, never in the URL, so it cannot show up in a log line or a redirect.
- `reqwest` runs with redirects disabled, a 5 second connect timeout, a 10 second total timeout and no system proxy.
- Identifiers that end up in URLs pass `validate_safe_identifier`, and API keys pass `validate_api_key_format`.
- Image URLs must be HTTPS on `yt3.ggpht.com`, `yt3.googleusercontent.com` or `i.ytimg.com`. Rust checks this, the UI checks it again, and the CSP enforces it.
- Error text from YouTube is never shown as received. It is mapped to a fixed sentence, and anything that is passed through is scrubbed of keys and tokens first.

## Known limits

- ETag and quota. Conditional requests save bandwidth and skip two calls. Google's overview says every request costs at least one unit and does not say a `304` is free, so the app counts a `304` as one unit in its own estimate. Check the Cloud Console quota page for the real figure.
- The quota counter resets at a fixed UTC-8 offset, so it is an hour off during daylight saving time.
- The notch layout has been exercised in simulation and unit tests, not on hardware.
- With more than one display, the app picks the built-in notched screen, otherwise the primary one. Hot-plugging has not been tested.
- Releases are unsigned.

## Tests

Rust, in `src-tauri/`:

- `tests/telemetry.rs` runs the client against a local fake YouTube server: ETag and `304` behavior, the key in a header, error sentences, redirects, cleaning of text, and failure handling.
- `tests/vault.rs` covers encryption on disk, restarts, tampering, file modes, atomic writes and input rules.
- `tests/security.rs` covers secret scrubbing and the validators, and audits the project files: the CSP, the capability, the list of commands, the plugins and the sources.
- `tests/geometry.rs` covers window bounds, the hover target and the dwell rule.
- `tests/release.rs` checks what must agree before a tag: the versions, the release workflow and the Homebrew cask, pinned actions, the plist. It also fails if `.gitignore` hides a project file or any file holds something shaped like a real key.
- Unit tests next to the code cover screen layout, error messages and the `NSPanel` size check.

Web, in `src/__tests__/`:

- `NotchDashboard.test.tsx`, `layout.test.tsx` and `flows.test.tsx` render the app against a fake backend: stages, hover, tabs, setup, first run, notch layout, keyboard focus and window sizing.
- `lib.test.ts` covers the helpers: number and time formatting, credential detection, the image URL check, the metric catalog and the theme.
