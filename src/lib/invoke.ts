import { invoke } from "@tauri-apps/api/core";

export type {
  AdminPanelInfo,
  AdminSessionSnapshot,
  AdminSessionState,
  ExpiredActionStatus,
  FrontendConfig,
  PauseReason,
} from "./bindings";

function invokeCommand<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  return invoke<T>(command, args);
}

export function getConfig() {
  return invokeCommand<import("./bindings").FrontendConfig>("get_config");
}

export function finishSetup() {
  return invokeCommand<void>("finish_setup");
}

export function updateSettings(
  timeoutMinutes: number,
  warningMinutes: number,
  action: string,
  autostartEnabled: boolean,
) {
  return invokeCommand<void>("update_settings", {
    timeoutMinutes,
    warningMinutes,
    action,
    autostartEnabled,
  });
}

export function isFirstRun() {
  return invokeCommand<boolean>("is_first_run");
}

export function setupPassword(password: string, timeoutMinutes: number) {
  return invokeCommand<string>("setup_password", { password, timeoutMinutes });
}

export function verifyPassword(password: string) {
  return invokeCommand<boolean>("verify_password", { password });
}

export function verifyRecoveryKey(key: string) {
  return invokeCommand<boolean>("verify_recovery_key", { key });
}

export function resetPasswordWithRecovery(key: string, newPassword: string) {
  return invokeCommand<boolean>("reset_password_with_recovery", {
    key,
    newPassword,
  });
}

export function changePassword(current: string, newPassword: string) {
  return invokeCommand<boolean>("change_password", {
    current,
    newPassword,
  });
}

export function startTimer() {
  return invokeCommand<void>("start_timer");
}

export function clearTimer() {
  return invokeCommand<void>("clear_timer");
}

export function pauseTimer() {
  return invokeCommand<void>("pause_timer");
}

export function resumeTimer() {
  return invokeCommand<void>("resume_timer");
}

export function getRemainingSeconds() {
  return invokeCommand<number | null>("get_remaining_seconds");
}

export function markWarningNotificationSent() {
  return invokeCommand<void>("mark_warning_notification_sent");
}

export function executeExpiredAction() {
  return invokeCommand<import("./bindings").ExpiredActionStatus>(
    "execute_expired_action",
  );
}

export function getAdminPanelInfo() {
  return invokeCommand<import("./bindings").AdminPanelInfo>(
    "get_admin_panel_info",
  );
}

export function quitApp() {
  return invokeCommand<void>("quit_app");
}
