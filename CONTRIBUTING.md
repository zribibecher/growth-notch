# Contributing to Growth Notch

Thanks for taking a look. Growth Notch does one thing: it shows a creator's numbers in the notch and stays out of the way otherwise. Changes that keep it quiet and small are the easiest to merge.

## Get it running

You need Node 22.12 or newer, Rust 1.90 or newer, and the Xcode command line tools. You do not need an Apple Developer account.

```bash
npm install
npm run tauri dev
```

If your Mac has no notch, `NOTCH_SIM=1 npm run tauri dev` pretends it has one so you can work on that layout.

The first launch opens the setup card. To start over, quit the app and delete `~/Library/Application Support/com.techuntangled.growth-notch`. That folder holds the encrypted vault, so only do it with a test key.

## How a change travels

In short:

- The React hooks and components call Rust commands (`commands.rs`) and never the network.
- `telemetry.rs` makes the HTTPS calls to the YouTube Data API and `credentials.rs` keeps the key.
- `watcher.rs` watches the cursor and sends `notch://hover` to the UI, and `screen.rs` sends the layout.

The web view makes no API calls. Anything that needs a secret or an HTTP request goes through a Rust command, and each command must be listed in `build.rs` and `capabilities/default.json` (a test fails if the lists drift apart). The cursor watcher is what tells the UI when to unfold, because WebKit does not report hover to a window that isn't focused.

## Before you open a pull request

These should all pass on your machine. CI runs the same commands.

```bash
npm run format:check
npx tsc --noEmit
npm test
cd src-tauri
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

`cargo test` starts a mock HTTP server on a local port, so it will fail in a sandbox that blocks listening sockets.

New behavior needs a test next to the closest existing one:

| Kind of change | Where the test goes |
|---|---|
| UI behavior | `src/__tests__/` |
| A pure helper (formatting, metrics, theme) | `src/__tests__/lib.test.ts` |
| Network and error handling | `src-tauri/tests/telemetry.rs` |
| The vault | `src-tauri/tests/vault.rs` |
| Validation, scrubbing, what the web view may do | `src-tauri/tests/security.rs` |
| Window bounds and hover rules | `src-tauri/tests/geometry.rs` |
| Release facts, `.gitignore`, secret-shaped strings | `src-tauri/tests/release.rs` |

For anything visible, attach a screenshot or a short clip. Check all three stages (idle, peek, dashboard) in both themes. If you touch sizes or timings, update `docs/DESIGN.md` too.

## Style

- Comments say why, not what. Delete any comment the next reader wouldn't miss.
- Run `npm run format` before you commit web code. Rust follows `cargo fmt`.
- A few repeated lines are better than a helper made too early.
- Keep the network surface tiny. A new host needs a CSP change, a README change and a reason.
- Never log or show a secret. Error text goes through `sanitize_sensitive_text`.
- Write commit messages in the imperative: "Add milestone badge", not "Added milestone badge".

## Known advisories

Shipped dependencies are clean: `npm audit --omit=dev` finds nothing, and CI runs it. Two groups of findings remain, and neither affects the app you install.

- `npm audit` reports seven findings, all in the build tooling of Tailwind 3 (`braces`, `micromatch`, `fast-glob`, `chokidar`, `postcss-nested` and `postcss-selector-parser`). They need hostile glob patterns or hostile CSS fed to the build, and none of that code ships. The only fix is Tailwind 4, which targets Safari 16.4 and newer and says it will not work in older browsers. Moving to it means deciding first how old a macOS the app still supports.
- `Cargo.lock` lists five advisories for crates that only Linux builds of Tauri use: `glib` 0.18 (unsound iterator, fixed in 0.20), and `proc-macro-error`, `zvariant`, `zvariant_derive` and `zbus_names` (marked unmaintained). `cargo tree --target aarch64-apple-darwin -i <crate>` finds none of them in the macOS build. Tauri pulls them in, so the fix has to come from upstream.

## Good first issues

Look for the `good first issue` label. Typical ones: a new metric card, number formatting for another locale, a new theme, more tests, and documentation.

## Reporting a bug

Use the bug template. Notch problems usually depend on the display setup, so say which Mac and macOS version you have, and whether you use the built-in screen, an external monitor, or both.

To get logs, quit Growth Notch and run it from Terminal:

```bash
/Applications/GrowthNotch.app/Contents/MacOS/growth-notch
```

Remove any API key before pasting the output.

## Security issues

Please do not file these as public issues. Follow [`SECURITY.md`](SECURITY.md).
