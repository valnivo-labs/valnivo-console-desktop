# Valnivo Crew desktop

The Valnivo Crew app for macOS and Windows: a small [Tauri 2](https://tauri.app) window on
<https://console.valnivo.eu/ui/>, the same Crew as in the browser, signed in with the same account. The app holds no server
code and no keys; everything it shows comes from the website.

- **Download:** [Releases](https://github.com/valnivo-labs/valnivo-crew-desktop/releases): a universal `.dmg` for macOS
  (Apple silicon and Intel, macOS 11+) and a `-setup.exe` for Windows 10 and 11.
- **Build:** GitHub Actions, workflow **Desktop apps** (run it by hand, or push a tag `desktop-v<version>` to build and
  publish a release).
- **Source and details:** [`desktop/`](desktop/) and its [README](desktop/README.md) (first-open steps while the apps are
  not signed, building on your own machine).
