import { invoke } from "@tauri-apps/api/core";

export interface AppConfig {
  password_hash: string;
  recovery_key_hash: string;
  timeout_minutes: number;
  warning_minutes: number;
  action: string;
  autostart_enabled: boolean;
  first_run_complete: boolean;
  timer_start_timestamp: number | null;
}

export async function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_config");
}

export async function saveConfig(config: AppConfig): Promise<void> {
  return invoke("save_config_cmd", { config });
}

export async function isFirstRun(): Promise<boolean> {
  return invoke<boolean>("is_first_run");
}

export async function setupPassword(
  password: string,
  timeout_minutes: number
): Promise<string> {
  return invoke<string>("setup_password", {
    password,
    timeout_minutes,
  });
}

export async function verifyPassword(password: string): Promise<boolean> {
  return invoke<boolean>("verify_password", { password });
}

export async function verifyRecoveryKey(key: string): Promise<boolean> {
  return invoke<boolean>("verify_recovery_key", { key });
}

export async function changePassword(
  current: string,
  new_password: string
): Promise<boolean> {
  return invoke<boolean>("change_password", {
    current,
    new_password,
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

export async function getRemainingSeconds(): Promise<number | null> {
  return invoke<number | null>("get_remaining_seconds");
}

export async function quitApp(): Promise<void> {
  return invoke("quit_app");
}
