import { invoke } from "@tauri-apps/api/core";

export type PauseReason = "manual" | "system";

export interface AppConfig {
  timeout_minutes: number;
  warning_minutes: number;
  action: string;
  autostart_enabled: boolean;
  first_run_complete: boolean;
  session_start_pending: boolean;
  timer_start_timestamp: number | null;
  timer_paused_at: number | null;
  pause_reason: PauseReason | null;
  warning_notification_sent: boolean;
}

export async function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_config");
}

export async function finishSetup(): Promise<void> {
  return invoke("finish_setup");
}

export async function updateSettings(
  timeoutMinutes: number,
  warningMinutes: number,
  action: string,
  autostartEnabled: boolean,
): Promise<void> {
  return invoke("update_settings", {
    timeoutMinutes,
    warningMinutes,
    action,
    autostartEnabled,
  });
}

export async function isFirstRun(): Promise<boolean> {
  return invoke<boolean>("is_first_run");
}

export async function setupPassword(
  password: string,
  timeoutMinutes: number,
): Promise<string> {
  return invoke<string>("setup_password", {
    password,
    timeoutMinutes,
  });
}

export async function verifyPassword(password: string): Promise<boolean> {
  return invoke<boolean>("verify_password", { password });
}

export async function verifyRecoveryKey(key: string): Promise<boolean> {
  return invoke<boolean>("verify_recovery_key", { key });
}

export async function resetPasswordWithRecovery(
  key: string,
  newPassword: string,
): Promise<boolean> {
  return invoke<boolean>("reset_password_with_recovery", {
    key,
    newPassword,
  });
}

export async function changePassword(
  current: string,
  newPassword: string,
): Promise<boolean> {
  return invoke<boolean>("change_password", {
    current,
    newPassword,
  });
}

export async function executeShutdown(action: string): Promise<void> {
  return invoke("execute_shutdown", { action });
}

export async function startTimer(): Promise<void> {
  return invoke("start_timer");
}

export async function clearTimer(): Promise<void> {
  return invoke("clear_timer");
}

export async function clearTimerForNextLogin(): Promise<void> {
  return invoke("clear_timer_for_next_login");
}

export async function pauseTimer(): Promise<void> {
  return invoke("pause_timer");
}

export async function resumeTimer(): Promise<void> {
  return invoke("resume_timer");
}

export async function getRemainingSeconds(): Promise<number | null> {
  return invoke<number | null>("get_remaining_seconds");
}

export async function markWarningNotificationSent(): Promise<void> {
  return invoke("mark_warning_notification_sent");
}

export async function quitApp(): Promise<void> {
  return invoke("quit_app");
}
