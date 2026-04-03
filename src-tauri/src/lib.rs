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
mod session;
mod shutdown;
mod windows_session;

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
    let is_autostart_launch = std::env::args().any(|arg| arg == "--minimized");

    // Set up panic hook for logging
    std::panic::set_hook(Box::new(|panic_info| {
        log_error(&format!("PANIC: {}", panic_info));
    }));

    log_error("Starting Sessionizer...");

        let specta_builder = tauri_specta::Builder::<tauri::Wry>::new()
            .commands(tauri_specta::collect_commands![
                commands::get_config,
                commands::finish_setup,
                commands::update_settings,
                commands::is_first_run,
                commands::setup_password,
                commands::verify_password,
                commands::verify_recovery_key,
                commands::reset_password_with_recovery,
                commands::change_password,
                commands::execute_shutdown,
                commands::start_timer,
                commands::clear_timer,
                commands::clear_timer_for_next_login,
                commands::pause_timer,
                commands::resume_timer,
                commands::get_remaining_seconds,
                commands::mark_warning_notification_sent,
                commands::quit_app,
            ]);
            
        #[cfg(debug_assertions)]
        specta_builder
            .export(tauri_specta::ts::ExportConfiguration::new().bigint(tauri_specta::ts::BigIntExportBehavior::Number), "../src/lib/bindings.ts")
            .expect("Failed to export typescript bindings");

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
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            log_error("Running setup...");

            if let Err(e) = session::apply_startup_policy(is_autostart_launch) {
                log_error(&format!("Failed to apply startup policy: {}", e));
            }

            let settings_item = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let relock_item = MenuItem::with_id(app, "relock", "Re-lock", true, None::<&str>)?;
            let resume_item =
                MenuItem::with_id(app, "resume-session", "Resume Session", true, None::<&str>)?;
            let about_item = MenuItem::with_id(app, "about", "About", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let menu = Menu::with_items(
                app,
                &[
                    &settings_item,
                    &relock_item,
                    &resume_item,
                    &about_item,
                    &quit_item,
                ],
            )?;

            if let Some(window) = app.get_webview_window("main") {
                if let Err(e) = windows_session::install(&window) {
                    log_error(&format!("Failed to install Windows session hook: {}", e));
                }
            }

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
                        "resume-session" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.emit("resume-session", ());
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

#[cfg(test)]
mod bindings {
    use super::*;
    
    #[test]
    fn export_bindings() {
        let specta_builder = tauri_specta::Builder::<tauri::Wry>::new()
            .commands(tauri_specta::collect_commands![
                commands::get_config,
                commands::finish_setup,
                commands::update_settings,
                commands::is_first_run,
                commands::setup_password,
                commands::verify_password,
                commands::verify_recovery_key,
                commands::reset_password_with_recovery,
                commands::change_password,
                commands::execute_shutdown,
                commands::start_timer,
                commands::clear_timer,
                commands::clear_timer_for_next_login,
                commands::pause_timer,
                commands::resume_timer,
                commands::get_remaining_seconds,
                commands::mark_warning_notification_sent,
                commands::quit_app,
            ]);
            
        specta_builder
            .export(tauri_specta::ts::ExportConfiguration::new().bigint(tauri_specta::ts::BigIntExportBehavior::Number), "../src/lib/bindings.ts")
            .expect("Failed to export typescript bindings");
    }
}
