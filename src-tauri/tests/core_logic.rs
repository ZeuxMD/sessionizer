#![allow(dead_code)]

#[path = "../src/config.rs"]
mod config;

#[path = "../src/session.rs"]
mod session;

use config::{AppConfig, PauseReason};

#[test]
fn adjust_remaining_seconds_adds_time_for_running_sessions() {
    let mut config = AppConfig {
        timeout_minutes: 60,
        warning_minutes: 5,
        timer_start_timestamp: Some(1_000),
        ..AppConfig::default()
    };

    let remaining = session::adjust_remaining_seconds(&mut config, 5 * 60, 1_600);

    assert_eq!(remaining, Some(3_300));
    assert_eq!(config.timer_start_timestamp, Some(1_300));
}

#[test]
fn adjust_remaining_seconds_preserves_paused_session_state() {
    let mut config = AppConfig {
        timeout_minutes: 60,
        warning_minutes: 5,
        timer_start_timestamp: Some(1_000),
        timer_paused_at: Some(1_900),
        pause_reason: Some(PauseReason::Manual),
        ..AppConfig::default()
    };

    let remaining = session::adjust_remaining_seconds(&mut config, -10 * 60, 2_500);

    assert_eq!(remaining, Some(2_100));
    assert_eq!(config.timer_start_timestamp, Some(400));
    assert_eq!(config.timer_paused_at, Some(1_900));
    assert_eq!(config.pause_reason, Some(PauseReason::Manual));
}

#[test]
fn restart_session_clears_pause_and_warning_state() {
    let mut config = AppConfig {
        warning_notification_sent: true,
        timer_start_timestamp: Some(10),
        timer_paused_at: Some(20),
        pause_reason: Some(PauseReason::System),
        ..AppConfig::default()
    };

    session::restart_session(&mut config, 55);

    assert_eq!(config.timer_start_timestamp, Some(55));
    assert_eq!(config.timer_paused_at, None);
    assert_eq!(config.pause_reason, None);
    assert!(!config.warning_notification_sent);
}
