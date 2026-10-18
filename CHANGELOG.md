# Changelog

All notable changes are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

The first public release. This section becomes `[0.1.0]`, with the date, when the tag is cut.

### Added
- The notch widget with three stages: an invisible idle state, a hover peek (subscribers, likes, comments) and a dashboard that opens on click, with a spring unfold.
- A dashboard with the subscriber count and its change since launch, likes and comments for the last 10 uploads (each in its own colour), the latest upload with thumbnail and age, 12 optional metrics, and light and dark themes.
- A layout that adapts to a camera notch. `NOTCH_SIM=1` simulates one on a Mac without a notch.
- Hover detection that works while the app is unfocused. The pill reacts only over its own visible box, needs about 125 ms of rest before it unfolds, and ignores the mouse everywhere else, so it never blocks clicks or scrolling in the page underneath.
- Keyboard focus only while the Setup field is open, so typing stays with the app you are using.
- Drawing over other apps' full-screen Spaces, using a non-activating panel.
- A first-run welcome and a one-field setup that tells an API key from a channel ID.
- A menu bar icon with Show Dashboard, Refresh Now and Quit.
- An encrypted local vault (ChaCha20-Poly1305, atomic writes, private directory).
- Error messages written for people, such as "YouTube rejected the API key. Paste it again in Setup." The previous numbers stay on screen when a refresh fails.
- A light haptic tick on supported trackpads when the notch unfolds.
- Reduced-motion support, ARIA labels and focus rings.

### Security
- A strict Content Security Policy, redirects disabled, and no system proxy.
- The API key travels in a request header, never in a URL.
- The web view can call seven commands and nothing else. Tauri's default permissions are not used.
- API keys and tokens are scrubbed from error text, and control and direction-override characters are stripped from text that comes from YouTube.
- The vault directory is private to your user (`0700`) and its files are `0600`.
- `rustls` is at 0.23.46, so the lockfile has no advisories for it.
- The unused OAuth and deep-link code was removed.

### Known limits
- Releases are not code signed or notarized.
- The notch layout has been checked in simulation only.
- A `304` answer counts as one unit in the app's quota estimate, because Google does not say it is free.
