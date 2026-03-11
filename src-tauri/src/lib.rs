use std::fs::OpenOptions;
use std::io::Write;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};

mod commands;
mod config;
mod password;
mod shutdown;

fn log_error(msg: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("sessionizer_error.log")
    {
        let _ = writeln!(file, "{}", msg);
    }
    eprintln!("{}", msg);
}

pub fn run() {
    // Set up panic hook for logging
    std::panic::set_hook(Box::new(|panic_info| {
        log_error(&format!("PANIC: {}", panic_info));
    }));

    log_error("Starting Sessionizer...");

    let result = tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            log_error("Single instance triggered");
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config_cmd,
            commands::is_first_run,
            commands::setup_password,
            commands::verify_password,
            commands::verify_recovery_key,
            commands::change_password,
            commands::execute_shutdown,
            commands::start_timer,
            commands::clear_timer,
            commands::pause_timer,
            commands::resume_timer,
            commands::get_remaining_seconds,
            commands::quit_app,
        ])
        .setup(|app| {
            log_error("Running setup...");

            let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let relock_item = MenuItem::with_id(app, "relock", "Re-lock", true, None::<&str>)?;
            let about_item = MenuItem::with_id(app, "about", "About", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[&settings_item, &relock_item, &about_item, &quit_item],
            )?;

            // Try to create tray icon, but don't fail if it doesn't work
            match TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(|app, event| {
                    let id = event.id.as_ref();
                    match id {
                        "settings" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.emit("show-settings", ());
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "relock" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.emit("re-lock", ());
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "about" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.emit("show-about", ());
                                let _ = window.show();
                            }
                        }
                        "quit" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.emit("quit-app", ());
                                let _ = window.show();
                            }
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)
            {
                Ok(_tray) => log_error("Tray icon created successfully"),
                Err(e) => log_error(&format!("Failed to create tray icon: {}", e)),
            }

            log_error("Setup complete");
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!());

    match result {
        Ok(_) => log_error("App exited normally"),
        Err(e) => log_error(&format!("App error: {}", e)),
    }
}
