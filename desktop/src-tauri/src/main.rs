// Card 739: the Valnivo Console as an app for macOS and Windows. One window on https://console.valnivo.eu/ui/ (the same Console
// as the browser, signed in with the same account), remembered size and place, one copy at a time, and links to anywhere else
// open in the normal browser. The page gets almost no access to the computer: only Valnivo Crew's own site may show a
// notification and put the waiting count on the Dock icon (capabilities/default.json); nothing else is exposed.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{Manager, Url, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

const HOME: &str = "https://console.valnivo.eu/ui/";

/// Pages that stay in the app window: the Console itself and Valnivo's own hosts (the shared desktop view); local schemes
/// the page uses for downloads and previews.
fn stays_inside(url: &Url) -> bool {
    match url.scheme() {
        "about" | "blob" | "data" => true,
        "https" => url.host_str().is_some_and(|h| h == "valnivo.eu" || h.ends_with(".valnivo.eu")),
        _ => false,
    }
}

fn main() {
    tauri::Builder::default()
        // a second launch brings the open window forward instead of starting another copy
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let home: Url = HOME.parse().expect("the Console address");
            let handle = app.handle().clone();
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(home))
                .title("Valnivo Crew")
                .inner_size(1280.0, 820.0)
                .min_inner_size(720.0, 520.0)
                .on_navigation(move |url| {
                    if stays_inside(url) {
                        return true;
                    }
                    if matches!(url.scheme(), "https" | "http" | "mailto") {
                        let _ = handle.opener().open_url(url.as_str(), None::<&str>);
                    }
                    false
                })
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("the Valnivo Crew app could not start");
}
