# Security policy

## Reporting a vulnerability

Use GitHub's private reporting: open the Security tab of [zribibecher/growth-notch](https://github.com/zribibecher/growth-notch/security/advisories/new) and choose "Report a vulnerability". Please don't open a public issue for a security problem.

You should hear back within a few days. Growth Notch has one maintainer, so there is no fixed timeline for a fix, but anything that touches credentials goes first.

## What the app handles

- A YouTube Data API key and a channel ID, both supplied by the user.
- Public channel and video statistics returned by `www.googleapis.com`, and the avatar and thumbnails that YouTube's image hosts serve.

There is no account system, no backend and no analytics.

## How secrets are stored

- `vault.enc` holds the key and channel ID, encrypted with ChaCha20-Poly1305. It lives in the app's data directory (mode `0700`) with mode `0600` and is replaced atomically on every write.
- The encryption key is a random 32-byte file, `.vault.key`, in the same directory, also mode `0600`.
- That setup stops casual reads and accidental commits. It does not stop malware that runs as the same macOS user, and the macOS Keychain is not used.
- Secrets sit in memory in buffers that are zeroed on drop. The API key is sent in a request header, not in a URL, and it is scrubbed from error text before that text leaves the Rust side.

If you think this threat model is too weak, please say so. It will be tracked as a design discussion rather than a vulnerability.

## In scope

- Anything that leaks the API key through logs, error text, the web view or the network.
- Any way to make the app send credentials to a host other than `www.googleapis.com`.
- Web view escapes, such as a CSP bypass or an unsafe IPC command. The web view may call seven commands, listed in `src-tauri/build.rs`.
- Memory-safety bugs in the `unsafe` AppKit code in `src-tauri/src/panel.rs`.

## Out of scope

- Attacks that already need code execution as the user.
- Exhausting your own YouTube API quota.
- Missing code signing. Releases are unsigned for now, and the README says so.
- Advisories in Tailwind 3's build tooling (seven `npm audit` findings) and in crates that only Linux builds of Tauri use. Nothing from them ships in the macOS app. See `CONTRIBUTING.md`.

## Supported versions

Only the latest release.
