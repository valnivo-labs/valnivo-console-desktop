# Valnivo Crew for macOS and Windows

The Console as an app: one window on <https://console.valnivo.eu/ui/>, the same Console as in the browser, signed in with the
same account. It is a [Tauri 2](https://tauri.app) shell (Rust, the system's own web view: WebKit on macOS, WebView2 on Windows),
so it is small. The page updates the next time the window loads. From 0.10.0 the window can update itself: Settings shows the installed version, Check for updates, and Update now. Later waits 24 hours before asking again. The check also runs in the background about every 6 hours and does not block launch.

People on 0.9.0 install 0.10.0 once from the downloads page (that build has no updater). After that, Update now downloads the signed package, checks the signature, installs it and restarts the app. If that fails, the same downloads page is the recovery path. Linux is not a target. The updater signature is not an Apple or Windows certificate, so the first open can still show a system warning.

What the app adds over the browser tab: its own window and Dock / taskbar icon, remembered size and place, one copy at a time
(opening it again brings the window forward), and links to other sites open in your normal browser. The page may show a notification, set the Dock badge, read the app version, and install a signed update. No other command is exposed.

## Getting the apps

The apps are built in the **public** repository [valnivo-labs/valnivo-crew-desktop](https://github.com/valnivo-labs/valnivo-crew-desktop),
where GitHub Actions minutes for macOS and Windows are free (this private repo's Actions billing does not cover them).
Download them from its [Releases](https://github.com/valnivo-labs/valnivo-crew-desktop/releases).

This folder stays the source. `scripts/desktop-publish.sh` copies `desktop/` there, and `desktop/ci/desktop.yml` into the public repo's workflow. Everything in this
folder becomes public, so keep it to the app shell, never keys or server code.

```
scripts/desktop-publish.sh            # sync only
scripts/desktop-publish.sh --build    # sync and build by hand: the run's artifacts
scripts/desktop-publish.sh --tag      # sync and tag desktop-v<package.json version>: build and publish a Release
```

Each run's artifacts are:

- `valnivo-console-macos`: `Valnivo Crew_<version>_universal.dmg` (Apple silicon and Intel, macOS 11 or later) and `Valnivo Crew.app.tar.gz` plus its signature (what the updater installs)
- `valnivo-console-windows`: `Valnivo Crew_<version>_x64-setup.exe` (Windows 10 and 11; installs for the current user) plus its signature
- `latest.json` on the release, which the app reads

The build refuses to run until the public repository has the updater signing key in its Actions secrets (`TAURI_SIGNING_PRIVATE_KEY`, and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` only if that key has a password). The private key is not in this folder. The matching public key is in `src-tauri/tauri.conf.json`. Losing the private key means installed copies cannot take a later update.

## Checking a download

Every released file carries a build provenance attestation (SLSA Build L2): GitHub signs that the file came from this
repository's `Desktop apps` workflow, at that commit. Check one before installing it:

```
gh attestation verify "Valnivo-Crew-macOS.dmg" -R valnivo-labs/valnivo-crew-desktop
```

The workflow lives only in the public repository (`.github/workflows/desktop.yml`); its outside actions are pinned to exact commits.

## Not signed yet

Until the owner's certificates are added, both systems warn on first open:

- **macOS** (ad-hoc signed): "Apple could not verify…". Press **Done**, then **System Settings → Privacy & Security → Open
  Anyway**. Or, once: `xattr -dr com.apple.quarantine "/Applications/Valnivo Crew.app"`.
- **Windows** (unsigned): SmartScreen. **More info → Run anyway**.

Signing needs the owner's Apple Developer ID (with notarisation) and a Windows code-signing certificate; both are added as
secrets of the public build repository and its workflow then signs. Nothing else changes.

## Windows title bar

The page background is `#0a0b0d` and the text is `#e9e9ea`. On Windows 11 the native caption, its text and its border are set
to those colors through the Desktop Window Manager. If Windows refuses a color, the window stays open and only the dark caption
is requested. Windows 10 builds that predate the color attributes get that dark caption, not the exact page color. macOS is
unchanged. A compile on this desk is not a look at the real window.


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
