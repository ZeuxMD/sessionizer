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

#[test]
fn added_time_above_the_original_limit_counts_down_immediately() {
    let mut config = AppConfig {
        timeout_minutes: 60,
        ..AppConfig::default()
    };
    session::start_session(&mut config, 10_000);
    assert_eq!(
        session::adjust_remaining_seconds(&mut config, 30 * 60, 10_600),
        Some(80 * 60)
    );
    assert_eq!(
        session::get_remaining_seconds_at(&config, 11_200),
        Some(70 * 60)
    );
    assert_eq!(session::get_remaining_seconds_at(&config, 15_400), Some(0));
}

#[test]
fn added_time_above_the_limit_survives_pause_and_resume() {
    let mut config = AppConfig {
        timeout_minutes: 60,
        ..AppConfig::default()
    };
    session::start_session(&mut config, 10_000);
    session::pause_session(&mut config, PauseReason::Manual, 10_600);
    assert_eq!(
        session::adjust_remaining_seconds(&mut config, 30 * 60, 11_000),
        Some(80 * 60)
    );
    assert_eq!(
        session::get_remaining_seconds_at(&config, 12_000),
        Some(80 * 60)
    );
    session::resume_session(&mut config, 12_000);
    assert_eq!(
        session::get_remaining_seconds_at(&config, 12_060),
        Some(79 * 60)
    );
}
