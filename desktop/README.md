# Valnivo Crew for macOS and Windows

The Console as an app: one window on <https://console.valnivo.eu/ui/>, the same Console as in the browser, signed in with the
same account. It is a [Tauri 2](https://tauri.app) shell (Rust, the system's own web view: WebKit on macOS, WebView2 on Windows),
so it is small and updates itself with the website: a new Console release shows up the next time the window loads.

What the app adds over the browser tab: its own window and Dock / taskbar icon, remembered size and place, one copy at a time
(opening it again brings the window forward), and links to other sites open in your normal browser. The page gets no access to
the computer: no Tauri commands are exposed to it.

## Getting the apps

Download them from this repository's [Releases](https://github.com/valnivo-labs/valnivo-crew-desktop/releases).

GitHub Actions builds both: **Actions → Desktop apps → Run workflow** (or push a tag `desktop-v0.9.0`, which also publishes a
Release). When the run is done, its artifacts are:

- `valnivo-console-macos`: `Valnivo Crew_<version>_universal.dmg` (Apple silicon and Intel, macOS 11 or later)
- `valnivo-console-windows`: `Valnivo Crew_<version>_x64-setup.exe` (Windows 10 and 11; installs for the current user)

## Not signed yet

Until the owner's certificates are added, both systems warn on first open:

- **macOS** (ad-hoc signed): "Apple could not verify…". Press **Done**, then **System Settings → Privacy & Security → Open
  Anyway**. Or, once: `xattr -dr com.apple.quarantine "/Applications/Valnivo Crew.app"`.
- **Windows** (unsigned): SmartScreen. **More info → Run anyway**.

Signing needs the owner's Apple Developer ID (with notarisation) and a Windows code-signing certificate; both are added as
repository secrets and the workflow then signs. Nothing else changes.

## Building on your own machine

Needs Node 20+ and Rust (`brew install rust` on macOS; rustup on Windows).

```
cd desktop
npm ci
npx tauri build                 # this machine's app and installer
npx tauri dev                   # a window for trying changes
```

The icon is `icon/crew.svg` (drawn at 1024 px as `icon/crew-1024.png`; `npm run icons` makes every size from it). The address is `HOME` in `src-tauri/src/main.rs`; pages on
valnivo.eu hosts stay in the window, anything else opens in the browser.
