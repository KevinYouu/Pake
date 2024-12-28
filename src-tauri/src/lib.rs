#[cfg_attr(mobile, tauri::mobile_entry_point)]
mod app;
mod util;

use app::{invoke, window};
use invoke::{download_file, download_file_by_binary};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
use tauri_plugin_window_state::Builder as windowStatePlugin;
use util::{get_data_dir, get_pake_config};
use window::get_window;

pub fn run_app() {
    let (pake_config, tauri_config) = get_pake_config();

    let tauri_app = tauri::Builder::default();

    // Save the value of toggle_app_shortcut before pake_config is moved
    let activation_shortcut = pake_config.windows[0].activation_shortcut.clone();
    let init_fullscreen = pake_config.windows[0].fullscreen;

    let window_state_plugin = if init_fullscreen {
        windowStatePlugin::default()
            .with_state_flags(tauri_plugin_window_state::StateFlags::FULLSCREEN)
            .build()
    } else {
        windowStatePlugin::default().build()
    };

    tauri_app
        .plugin(window_state_plugin)
        .plugin(tauri_plugin_oauth::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_single_instance::init(|_, _, _| ()))
        .invoke_handler(tauri::generate_handler![
            download_file,
            download_file_by_binary
        ])
        .setup(move |app| {
            let data_dir = get_data_dir(app.app_handle(), tauri_config.clone());

            let _window = get_window(app, &pake_config, data_dir);

            // Prevent initial shaking
            _window.show().unwrap();

            if !activation_shortcut.is_empty() {
                let app_handle = app.app_handle().clone();
                let shortcut_hotkey = Shortcut::from_str(activation_shortcut.as_str()).unwrap();
                let last_triggered = Arc::new(Mutex::new(Instant::now()));

                app_handle
                    .plugin(
                        tauri_plugin_global_shortcut::Builder::new()
                            .with_handler({
                                let last_triggered = Arc::clone(&last_triggered);
                                move |app, event, _shortcut| {
                                    // Fixed the bug of tauri's hidden call, which caused repeated execution
                                    let now = Instant::now();
                                    let mut last = last_triggered.lock().unwrap();
                                    if now.duration_since(*last) < Duration::from_millis(500) {
                                        return;
                                    }
                                    *last = now;

                                    if shortcut_hotkey.eq(event) {
                                        let window = app.get_webview_window("pake").unwrap();
                                        let is_visible = window.is_visible().unwrap();

                                        match is_visible {
                                            true => {
                                                window.minimize().unwrap();
                                            }
                                            false => {
                                                window.unminimize().unwrap();
                                                window.set_focus().unwrap();
                                            }
                                        }
                                    }
                                }
                            })
                            .build(),
                    )
                    .expect("Error registering global evoke shortcuts!");

                app.global_shortcut().register(shortcut_hotkey)?;
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub fn run() {
    run_app()
}
