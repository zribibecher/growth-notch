# Design

The goal is a widget that vanishes until you want it and reads at a glance when you do. The model follows [codenotch](https://github.com/vinzdg/codenotch): compact, peek and expanded presentations that stay consistent with each other.

## Stages

| Stage | Trigger | Size without a notch | Notes |
|---|---|---|---|
| `idle` | default | 132 x 24, opacity 0 | Invisible. The window ignores mouse events. |
| `peek` | cursor reaches the notch | 248 x 32 | Subscribers, likes and comments. Refreshes in the background, at most every 15 seconds. |
| `expanded` | click on the pill, tray "Show Dashboard", or first run | 400 x 216 | Pulse, Metrics and Setup tabs. The close button or moving the cursor away closes it. |

On a notched screen, idle matches the housing. Peek is `notchWidth + 2 x 78` wide and as tall as the housing, with stats in the wings. Expanded adds the housing height as top padding.

## What reacts to the mouse

Only the visible pill. The window around it is larger so the spring can overshoot and the ears can show, but it ignores the mouse everywhere outside the pill, so it never blocks clicks or scrolling in the page underneath. The idle target is the small 132 x 24 pill (or the camera housing on a notched screen), and the cursor has to rest on it for about 125 ms before it unfolds. The panel takes keyboard focus only while the Setup tab is open.

## Motion

- The unfold uses a spring equivalent to SwiftUI `spring(response: 0.62, dampingFraction: 0.72)`, sampled into CSS `linear()` with a peak overshoot of 1.038. Browsers without `linear()` fall back to `cubic-bezier(0.22, 1, 0.36, 1)`.
- Content fades in after the shape opens: the peek layer after 60 ms, the dashboard after 120 ms.
- The native window shrinks 600 ms after a collapse so it never clips the retract animation.
- `prefers-reduced-motion` removes every transition.
- A light haptic tick fires when the notch unfolds, on Force Touch trackpads only.

## Look

- The surface is pure black (`#000`) with a faint sheen at the top, so it merges with a hardware notch.
- Each view has one large number (36 px), quiet 9 px uppercase labels, hairline dividers, and glass cards for secondary content.
- Each metric keeps its own colour (likes rose, comments sky, views emerald, and so on) so numbers can be told apart at a glance. YouTube red marks the live dot and the primary button.
- Numbers use tabular figures so they don't jitter as they change.
- A light theme exists. Idle stays black in both.
- There is no logo inside the dashboard. The app icon is a red growth arrow on a dark card, set on a lavender tile.

## Accessibility

- Icon buttons have `aria-label`s, the tabs use `role="tablist"` and `role="tab"`, and status text is a polite live region.
- Every control shows a focus ring.
- The peek icons carry `title` text explaining that likes and comments cover the last 10 uploads.

## Values that must stay in sync

- Window sizes: `src-tauri/src/notch.rs`
- Pill classes (`SHAPE`): `src/App.tsx`, and the open height `EXPANDED_HEIGHT` in `src/lib/screen.ts`
- Spring and timing: `src/styles.css`
- `WINDOW_SHRINK_DELAY_MS` in `src/hooks/useNotch.ts`, which must be longer than `--notch-dur`

Update this file when you change any of them.
