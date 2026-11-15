# Valnivo Crew for macOS and Windows

The Console as an app: one window on <https://console.valnivo.eu/ui/>, the same Console as in the browser, signed in with the
same account. It is a [Tauri 2](https://tauri.app) shell (Rust, the system's own web view: WebKit on macOS, WebView2 on Windows),
so it is small and updates itself with the website: a new Console release shows up the next time the window loads.

What the app adds over the browser tab: its own window and Dock / taskbar icon, remembered size and place, one copy at a time
(opening it again brings the window forward), and links to other sites open in your normal browser. The page gets no access to
the computer: no Tauri commands are exposed to it.

## Getting the apps

The apps are built in the **public** repository [valnivo-labs/valnivo-crew-desktop](https://github.com/valnivo-labs/valnivo-crew-desktop),
where GitHub Actions minutes for macOS and Windows are free (this private repo's Actions billing does not cover them).
Download them from its [Releases](https://github.com/valnivo-labs/valnivo-crew-desktop/releases).

This folder stays the source. `scripts/desktop-publish.sh` copies `desktop/` (and only `desktop/`) there: everything in this
folder becomes public, so keep it to the app shell, never keys or server code.

```
scripts/desktop-publish.sh            # sync only
scripts/desktop-publish.sh --build    # sync and build by hand: the run's artifacts
scripts/desktop-publish.sh --tag      # sync and tag desktop-v<package.json version>: build and publish a Release
```

Each run's artifacts are:

- `valnivo-console-macos`: `Valnivo Crew_<version>_universal.dmg` (Apple silicon and Intel, macOS 11 or later)
- `valnivo-console-windows`: `Valnivo Crew_<version>_x64-setup.exe` (Windows 10 and 11; installs for the current user)

## Not signed yet

Until the owner's certificates are added, both systems warn on first open:

- **macOS** (ad-hoc signed): "Apple could not verify…". Press **Done**, then **System Settings → Privacy & Security → Open
  Anyway**. Or, once: `xattr -dr com.apple.quarantine "/Applications/Valnivo Crew.app"`.
- **Windows** (unsigned): SmartScreen. **More info → Run anyway**.

Signing needs the owner's Apple Developer ID (with notarisation) and a Windows code-signing certificate; both are added as
secrets of the public build repository and its workflow then signs. Nothing else changes.

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
