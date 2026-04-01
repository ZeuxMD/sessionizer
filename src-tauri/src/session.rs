use chrono::Utc;

use crate::config::{load_config, save_config, AppConfig, PauseReason};

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub const ENDSESSION_LOGOFF_FLAG: usize = 0x8000_0000;
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub const PBT_APMSUSPEND_EVENT: u32 = 4;
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub const PBT_APMRESUMESUSPEND_EVENT: u32 = 7;
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub const PBT_APMRESUMEAUTOMATIC_EVENT: u32 = 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupAction {
    None,
    Start,
    Resume,
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionSignal {
    None,
    PauseSystem,
    ResumeSystem,
    Clear,
}

pub fn current_timestamp() -> u64 {
    Utc::now().timestamp().max(0) as u64
}

pub fn start_session(config: &mut AppConfig, now: u64) {
    config.timer_start_timestamp = Some(now);
    config.timer_paused_at = None;
    config.pause_reason = None;
    config.warning_notification_sent = false;
}

pub fn clear_session(config: &mut AppConfig) {
    config.timer_start_timestamp = None;
    config.timer_paused_at = None;
    config.pause_reason = None;
    config.warning_notification_sent = false;
}

pub fn pause_session(config: &mut AppConfig, reason: PauseReason, now: u64) -> bool {
    if config.timer_start_timestamp.is_some() && config.timer_paused_at.is_none() {
        config.timer_paused_at = Some(now);
        config.pause_reason = Some(reason);
        true
    } else {
        false
    }
}

pub fn resume_session(config: &mut AppConfig, now: u64) -> bool {
    if let (Some(start_timestamp), Some(paused_at)) =
        (config.timer_start_timestamp, config.timer_paused_at)
    {
        let pause_duration = now.saturating_sub(paused_at);
        config.timer_start_timestamp = Some(start_timestamp.saturating_add(pause_duration));
        config.timer_paused_at = None;
        config.pause_reason = None;
        true
    } else {
        false
    }
}

pub fn mark_warning_notification_sent(config: &mut AppConfig) -> bool {
    if config.warning_notification_sent {
        false
    } else {
        config.warning_notification_sent = true;
        true
    }
}

pub fn get_remaining_seconds_at(config: &AppConfig, now: u64) -> Option<u64> {
    config.timer_start_timestamp.map(|start_timestamp| {
        let total_seconds = config.timeout_minutes * 60;
        let effective_now = config.timer_paused_at.unwrap_or(now);
        let elapsed = effective_now.saturating_sub(start_timestamp);

        if elapsed >= total_seconds {
            0
        } else {
            total_seconds - elapsed
        }
    })
}

pub fn get_remaining_seconds(config: &AppConfig) -> Option<u64> {
    get_remaining_seconds_at(config, current_timestamp())
}

pub fn decide_startup_action(config: &AppConfig, is_autostart_launch: bool) -> StartupAction {
    if !config.first_run_complete {
        return StartupAction::None;
    }

    if config.timer_start_timestamp.is_some()
        && config.timer_paused_at.is_some()
        && config.pause_reason == Some(PauseReason::System)
    {
        return StartupAction::Resume;
    }

    if config.timer_start_timestamp.is_none() && is_autostart_launch {
        return StartupAction::Start;
    }

    StartupAction::None
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn classify_end_session(is_ending: bool, lparam_flags: usize) -> SessionSignal {
    if !is_ending {
        SessionSignal::None
    } else if lparam_flags & ENDSESSION_LOGOFF_FLAG != 0 {
        SessionSignal::PauseSystem
    } else {
        SessionSignal::Clear
    }
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn classify_power_broadcast(event: usize) -> SessionSignal {
    match event as u32 {
        PBT_APMSUSPEND_EVENT => SessionSignal::PauseSystem,
        PBT_APMRESUMEAUTOMATIC_EVENT | PBT_APMRESUMESUSPEND_EVENT => SessionSignal::ResumeSystem,
        _ => SessionSignal::None,
    }
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn apply_signal(config: &mut AppConfig, signal: SessionSignal, now: u64) -> bool {
    match signal {
        SessionSignal::None => false,
        SessionSignal::PauseSystem => pause_session(config, PauseReason::System, now),
        SessionSignal::ResumeSystem => {
            if config.pause_reason == Some(PauseReason::System) {
                resume_session(config, now)
            } else {
                false
            }
        }
        SessionSignal::Clear => {
            if config.timer_start_timestamp.is_none()
                && config.timer_paused_at.is_none()
                && config.pause_reason.is_none()
                && !config.warning_notification_sent
            {
                false
            } else {
                clear_session(config);
                true
            }
        }
    }
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn persist_signal(signal: SessionSignal) -> Result<(), String> {
    if signal == SessionSignal::None {
        return Ok(());
    }

    let mut config = load_config();
    if apply_signal(&mut config, signal, current_timestamp()) {
        save_config(&config)?;
    }

    Ok(())
}

pub fn apply_startup_policy(is_autostart_launch: bool) -> Result<(), String> {
    let mut config = load_config();

    match decide_startup_action(&config, is_autostart_launch) {
        StartupAction::None => Ok(()),
        StartupAction::Start => {
            start_session(&mut config, current_timestamp());
            save_config(&config)
        }
        StartupAction::Resume => {
            resume_session(&mut config, current_timestamp());
            save_config(&config)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    fn configured_app() -> AppConfig {
        AppConfig {
            first_run_complete: true,
            timeout_minutes: 60,
            ..AppConfig::default()
        }
    }

    #[test]
    fn startup_starts_on_autostart_without_existing_timer() {
        let config = configured_app();
        assert_eq!(decide_startup_action(&config, true), StartupAction::Start);
    }

    #[test]
    fn startup_stays_idle_on_manual_launch_without_timer() {
        let config = configured_app();
        assert_eq!(decide_startup_action(&config, false), StartupAction::None);
    }

    #[test]
    fn startup_resumes_system_paused_session() {
        let mut config = configured_app();
        start_session(&mut config, 100);
        pause_session(&mut config, PauseReason::System, 130);

        assert_eq!(decide_startup_action(&config, false), StartupAction::Resume);
        assert_eq!(decide_startup_action(&config, true), StartupAction::Resume);
    }

    #[test]
    fn startup_keeps_manual_pause_paused() {
        let mut config = configured_app();
        start_session(&mut config, 100);
        pause_session(&mut config, PauseReason::Manual, 130);

        assert_eq!(decide_startup_action(&config, true), StartupAction::None);
    }

    #[test]
    fn start_session_resets_warning_and_pause_metadata() {
        let mut config = configured_app();
        config.warning_notification_sent = true;
        config.pause_reason = Some(PauseReason::Manual);
        config.timer_paused_at = Some(100);

        start_session(&mut config, 42);

        assert_eq!(config.timer_start_timestamp, Some(42));
        assert_eq!(config.timer_paused_at, None);
        assert_eq!(config.pause_reason, None);
        assert!(!config.warning_notification_sent);
    }

    #[test]
    fn manual_pause_preserves_remaining_time_and_reason() {
        let mut config = configured_app();
        start_session(&mut config, 100);

        assert!(pause_session(&mut config, PauseReason::Manual, 160));
        assert_eq!(config.timer_paused_at, Some(160));
        assert_eq!(config.pause_reason, Some(PauseReason::Manual));
        assert_eq!(get_remaining_seconds_at(&config, 200), Some(3540));
    }

    #[test]
    fn system_pause_records_system_reason() {
        let mut config = configured_app();
        start_session(&mut config, 100);

        assert!(pause_session(&mut config, PauseReason::System, 130));
        assert_eq!(config.pause_reason, Some(PauseReason::System));
        assert_eq!(get_remaining_seconds_at(&config, 600), Some(3570));
    }

    #[test]
    fn resume_session_adjusts_start_timestamp_and_clears_pause_metadata() {
        let mut config = configured_app();
        start_session(&mut config, 100);
        pause_session(&mut config, PauseReason::Manual, 160);
        config.warning_notification_sent = true;

        assert!(resume_session(&mut config, 220));
        assert_eq!(config.timer_start_timestamp, Some(160));
        assert_eq!(config.timer_paused_at, None);
        assert_eq!(config.pause_reason, None);
        assert!(config.warning_notification_sent);
        assert_eq!(get_remaining_seconds_at(&config, 220), Some(3540));
    }

    #[test]
    fn clear_session_resets_session_metadata() {
        let mut config = configured_app();
        start_session(&mut config, 100);
        pause_session(&mut config, PauseReason::System, 160);
        config.warning_notification_sent = true;

        clear_session(&mut config);

        assert_eq!(config.timer_start_timestamp, None);
        assert_eq!(config.timer_paused_at, None);
        assert_eq!(config.pause_reason, None);
        assert!(!config.warning_notification_sent);
    }

    #[test]
    fn classify_logout_end_session_as_pause() {
        assert_eq!(
            classify_end_session(true, ENDSESSION_LOGOFF_FLAG),
            SessionSignal::PauseSystem
        );
    }

    #[test]
    fn classify_shutdown_end_session_as_clear() {
        assert_eq!(classify_end_session(true, 0), SessionSignal::Clear);
    }

    #[test]
    fn classify_suspend_event_as_pause() {
        assert_eq!(
            classify_power_broadcast(PBT_APMSUSPEND_EVENT as usize),
            SessionSignal::PauseSystem
        );
    }

    #[test]
    fn resume_signal_only_applies_to_system_paused_sessions() {
        let mut config = configured_app();
        start_session(&mut config, 100);
        pause_session(&mut config, PauseReason::Manual, 160);

        assert!(!apply_signal(&mut config, SessionSignal::ResumeSystem, 220));
        assert_eq!(config.timer_paused_at, Some(160));
        assert_eq!(config.pause_reason, Some(PauseReason::Manual));
    }
}
