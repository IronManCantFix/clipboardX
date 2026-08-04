# ClipboardX

![Build](https://github.com/IronManCantFix/clipboardX/actions/workflows/build.yml/badge.svg)

A modern, minimal clipboard manager for macOS — native feel, fully offline, low memory.

[简体中文](README.zh-CN.md)

## Features

- **Global shortcut** — summon the popover anywhere with `⌘⇧V` (customizable)
- **Clipboard history** — text, URLs, files, folders, images (PNG/JPEG/GIF/TIFF/HEIC)
- **Smart dedupe** — copying the same content again moves it to the top instead of duplicating
- **Instant search** — fuzzy or exact matching, case-insensitive, under 50ms
- **Pin** — keep important entries pinned on top (pinned items are never auto-cleaned)
- **Images** — thumbnail previews in the list, original images loaded on demand
- **Global dedupe** — duplicate content moves to front without repetition
- **Screen-aware positioning** — the popover stays on-screen near your cursor
- **Auto cleanup** — by count (default 15, up to 100) with pinned items protected
- **Excluded apps** — optionally ignore copies from password managers and terminals
- **Menu-bar app** — no Dock icon, launches at login (optional)
- **Fully offline** — no network, no telemetry, no cloud

## Tech Stack

- [Tauri 2](https://tauri.app) + Rust
- [Svelte 5](https://svelte.dev) (SvelteKit)
- [SQLite](https://sqlite.org) (rusqlite)
- [Vite](https://vite.dev)

## Development

Prerequisites: Node.js ≥ 20, Rust stable, Xcode Command Line Tools.

```bash
# Install frontend dependencies
npm install

# Start dev (auto-releases port 1420 on exit)
npm run dev:start

# Manual cleanup if port 1420 is stuck
npm run dev:stop
```

## Build

```bash
# Build the macOS app for Apple Silicon
npm run tauri build

# Output: src-tauri/target/release/bundle/
```

## CI Build

GitHub Actions builds the macOS (Apple Silicon) app on every tag push (`v*`) — see [`.github/workflows/build.yml`](.github/workflows/build.yml). Download the `.dmg` / `.app` from the release or workflow artifacts.

## Shortcuts

| Action | Shortcut |
|---|---|
| Show popover | `⌘⇧V` (customizable) |
| Paste & close | `⏎` |
| Navigate | `↑` / `↓` |
| Pin / unpin | `⌘P` |
| Delete | `⌫` |
| Switch tab | `⌘1` / `⌘2` |
| Close | `Esc` |

## Settings

Open via the tray icon or the gear button in the popover:

- History limit, auto-cleanup
- Appearance: opacity, window size (auto / custom), image preview height
- Search: on/off, fuzzy / exact
- Position: follow cursor / center screen
- Global shortcut recorder
- Launch at login
- Storage stats + clear image cache
- Update check

## Data

All data stays local:

```
~/Library/Application Support/com.clipboardx.app/
├── entries.db      # history database
└── images/         # captured images (PNG)
```

## License

[MIT](LICENSE)
