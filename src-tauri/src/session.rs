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
    Suspend,
    ResumeSystem,
    Logout,
    Shutdown,
}

pub fn current_timestamp() -> u64 {
    Utc::now().timestamp().max(0) as u64
}

pub fn start_session(config: &mut AppConfig, now: u64) {
    config.session_start_pending = false;
    config.timer_start_timestamp = Some(now);
    config.timer_paused_at = None;
    config.pause_reason = None;
    config.session_expired = false;
    config.warning_notification_sent = false;
}

pub fn clear_session(config: &mut AppConfig) {
    config.timer_start_timestamp = None;
    config.timer_paused_at = None;
    config.pause_reason = None;
    config.session_expired = false;
    config.warning_notification_sent = false;
}

pub fn restart_session(config: &mut AppConfig, now: u64) {
    clear_session(config);
    start_session(config, now);
}

pub fn pause_session(config: &mut AppConfig, reason: PauseReason, now: u64) -> bool {
    if config.timer_start_timestamp.is_some()
        && config.timer_paused_at.is_none()
        && !config.session_expired
    {
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
        if config.session_expired {
            return false;
        }

        let pause_duration = now.saturating_sub(paused_at);
        config.session_start_pending = false;
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

pub fn expire_session(config: &mut AppConfig) -> bool {
    if config.timer_start_timestamp.is_some() && !config.session_expired {
        config.session_expired = true;
        true
    } else {
        false
    }
}

pub fn get_remaining_seconds_at(config: &AppConfig, now: u64) -> Option<u64> {
    config.timer_start_timestamp.map(|start_timestamp| {
        let total_seconds = config.timeout_minutes * 60;
        let effective_now = if config.pause_reason.is_some() {
            config.timer_paused_at.unwrap_or(now)
        } else {
            now
        };
        let elapsed = effective_now.saturating_sub(start_timestamp);

        total_seconds.saturating_sub(elapsed)
    })
}

pub fn get_remaining_seconds(config: &AppConfig) -> Option<u64> {
    get_remaining_seconds_at(config, current_timestamp())
}

pub fn adjust_remaining_seconds(
    config: &mut AppConfig,
    delta_seconds: i64,
    now: u64,
) -> Option<u64> {
    if config.timer_start_timestamp.is_none() || config.session_expired {
        return None;
    }

    let start_timestamp = config.timer_start_timestamp?;
    let adjusted = if delta_seconds >= 0 {
        start_timestamp.saturating_add(delta_seconds as u64)
    } else {
        start_timestamp.saturating_sub(delta_seconds.unsigned_abs())
    };

    config.timer_start_timestamp = Some(adjusted);

    let remaining = get_remaining_seconds_at(config, now);
    if remaining.is_some_and(|seconds| seconds > config.warning_minutes * 60) {
        config.warning_notification_sent = false;
    }

    remaining
}

pub fn decide_startup_action(config: &AppConfig, is_autostart_launch: bool) -> StartupAction {
    if !config.first_run_complete {
        return StartupAction::None;
    }

    if config.session_expired {
        return StartupAction::None;
    }

    if config.timer_start_timestamp.is_some()
        && config.timer_paused_at.is_some()
        && config.pause_reason == Some(PauseReason::System)
    {
        return StartupAction::Resume;
    }

    if config.timer_start_timestamp.is_none()
        && config.autostart_enabled
        && (is_autostart_launch || config.session_start_pending)
    {
        return StartupAction::Start;
    }

    StartupAction::None
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn classify_end_session(is_ending: bool, lparam_flags: usize) -> SessionSignal {
    if !is_ending {
        SessionSignal::None
    } else if lparam_flags & ENDSESSION_LOGOFF_FLAG != 0 {
        SessionSignal::Logout
    } else {
        SessionSignal::Shutdown
    }
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn classify_power_broadcast(event: usize) -> SessionSignal {
    match event as u32 {
        PBT_APMSUSPEND_EVENT => SessionSignal::Suspend,
        PBT_APMRESUMEAUTOMATIC_EVENT | PBT_APMRESUMESUSPEND_EVENT => SessionSignal::ResumeSystem,
        _ => SessionSignal::None,
    }
}

fn mark_session_start_pending(config: &mut AppConfig) -> bool {
    if config.session_start_pending {
        false
    } else {
        config.session_start_pending = true;
        true
    }
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn apply_signal(config: &mut AppConfig, signal: SessionSignal, now: u64) -> bool {
    match signal {
        SessionSignal::None => false,
        SessionSignal::Suspend => {
            if config.session_expired {
                false
            } else {
                pause_session(config, PauseReason::System, now)
            }
        }
        SessionSignal::Logout => {
            if config.session_expired {
                let mut changed = false;

                if config.timer_start_timestamp.is_some()
                    || config.timer_paused_at.is_some()
                    || config.pause_reason.is_some()
                    || config.session_expired
                    || config.warning_notification_sent
                {
                    clear_session(config);
                    changed = true;
                }

                if mark_session_start_pending(config) {
                    changed = true;
                }

                return changed;
            }

            if pause_session(config, PauseReason::System, now) {
                true
            } else if config.timer_start_timestamp.is_none() {
                mark_session_start_pending(config)
            } else {
                false
            }
        }
        SessionSignal::ResumeSystem => {
            if config.session_expired {
                false
            } else if config.pause_reason == Some(PauseReason::System) {
                resume_session(config, now)
            } else {
                false
            }
        }
        SessionSignal::Shutdown => {
            let mut changed = false;

            if config.timer_start_timestamp.is_some()
                || config.timer_paused_at.is_some()
                || config.pause_reason.is_some()
                || config.session_expired
                || config.warning_notification_sent
            {
                clear_session(config);
                changed = true;
            }

            if mark_session_start_pending(config) {
                changed = true;
            }

            changed
        }
    }
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn persist_signal(signal: SessionSignal) -> Result<(), String> {
    if signal == SessionSignal::None {
        return Ok(());
    }

    let mut config = load_config()?;
    if apply_signal(&mut config, signal, current_timestamp()) {
        save_config(&config)?;
    }

    Ok(())
}

pub fn apply_startup_policy(is_autostart_launch: bool) -> Result<(), String> {
    let mut config = load_config()?;

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
    fn startup_starts_when_next_login_is_pending() {
        let config = configured_app();
        assert_eq!(decide_startup_action(&config, false), StartupAction::Start);
    }

    #[test]
    fn startup_stays_idle_on_manual_launch_without_timer() {
        let mut config = configured_app();
        config.session_start_pending = false;
        assert_eq!(decide_startup_action(&config, false), StartupAction::None);
    }

    #[test]
    fn startup_respects_disabled_autostart_even_if_pending() {
        let mut config = configured_app();
        config.autostart_enabled = false;

        assert_eq!(decide_startup_action(&config, true), StartupAction::None);
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
    fn startup_keeps_expired_sessions_locked() {
        let mut config = configured_app();
        start_session(&mut config, 100);
        expire_session(&mut config);

        assert_eq!(decide_startup_action(&config, true), StartupAction::None);
        assert_eq!(decide_startup_action(&config, false), StartupAction::None);
    }

    #[test]
    fn start_session_resets_warning_and_pause_metadata() {
        let mut config = configured_app();
        config.warning_notification_sent = true;
        config.session_start_pending = true;
        config.pause_reason = Some(PauseReason::Manual);
        config.session_expired = true;
        config.timer_paused_at = Some(100);

        start_session(&mut config, 42);

        assert_eq!(config.timer_start_timestamp, Some(42));
        assert_eq!(config.timer_paused_at, None);
        assert_eq!(config.pause_reason, None);
        assert!(!config.session_start_pending);
        assert!(!config.session_expired);
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
        assert!(!config.session_start_pending);
        assert!(config.warning_notification_sent);
        assert_eq!(get_remaining_seconds_at(&config, 220), Some(3540));
    }

    #[test]
    fn clear_session_resets_session_metadata() {
        let mut config = configured_app();
        start_session(&mut config, 100);
        pause_session(&mut config, PauseReason::System, 160);
        config.session_expired = true;
        config.warning_notification_sent = true;

        clear_session(&mut config);

        assert_eq!(config.timer_start_timestamp, None);
        assert_eq!(config.timer_paused_at, None);
        assert_eq!(config.pause_reason, None);
        assert!(!config.session_expired);
        assert!(!config.warning_notification_sent);
    }

    #[test]
    fn expire_session_locks_active_session_without_clearing_timer() {
        let mut config = configured_app();
        start_session(&mut config, 100);

        assert!(expire_session(&mut config));
        assert!(config.session_expired);
        assert_eq!(get_remaining_seconds_at(&config, 4_000), Some(0));
        assert!(!pause_session(&mut config, PauseReason::Manual, 160));
        assert!(!resume_session(&mut config, 220));
    }

    #[test]
    fn classify_logout_end_session_as_pause() {
        assert_eq!(
            classify_end_session(true, ENDSESSION_LOGOFF_FLAG),
            SessionSignal::Logout
        );
    }

    #[test]
    fn classify_shutdown_end_session_as_clear() {
        assert_eq!(classify_end_session(true, 0), SessionSignal::Shutdown);
    }

    #[test]
    fn classify_suspend_event_as_pause() {
        assert_eq!(
            classify_power_broadcast(PBT_APMSUSPEND_EVENT as usize),
            SessionSignal::Suspend
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

    #[test]
    fn logout_without_active_session_marks_next_login_pending() {
        let mut config = configured_app();
        config.session_start_pending = false;

        assert!(apply_signal(&mut config, SessionSignal::Logout, 220));
        assert!(config.session_start_pending);
    }

    #[test]
    fn logout_clears_expired_session_and_marks_next_login_pending() {
        let mut config = configured_app();
        config.session_start_pending = false;
        start_session(&mut config, 100);
        expire_session(&mut config);

        assert!(apply_signal(&mut config, SessionSignal::Logout, 220));
        assert_eq!(config.timer_start_timestamp, None);
        assert!(config.session_start_pending);
        assert!(!config.session_expired);
    }

    #[test]
    fn suspend_without_active_session_leaves_pending_state_unchanged() {
        let mut config = configured_app();
        config.session_start_pending = false;

        assert!(!apply_signal(&mut config, SessionSignal::Suspend, 220));
        assert!(!config.session_start_pending);
    }

    #[test]
    fn shutdown_clears_session_and_marks_next_login_pending() {
        let mut config = configured_app();
        config.session_start_pending = false;
        start_session(&mut config, 100);
        config.session_expired = true;
        config.warning_notification_sent = true;

        assert!(apply_signal(&mut config, SessionSignal::Shutdown, 220));
        assert_eq!(config.timer_start_timestamp, None);
        assert_eq!(config.timer_paused_at, None);
        assert_eq!(config.pause_reason, None);
        assert!(!config.session_expired);
        assert!(!config.warning_notification_sent);
        assert!(config.session_start_pending);
    }

    #[test]
    fn legacy_paused_sessions_keep_counting_down() {
        let mut config = configured_app();
        config.timer_start_timestamp = Some(100);
        config.timer_paused_at = Some(130);
        config.pause_reason = None;

        assert_eq!(get_remaining_seconds_at(&config, 200), Some(3500));
    }
}
