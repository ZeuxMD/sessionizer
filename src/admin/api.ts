export type AdminSessionState =
  | "setup"
  | "unlocked"
  | "locked"
  | "paused"
  | "expired";

export type PauseReason = "manual" | "system";

export interface AdminPanelInfo {
  running: boolean;
  listen_address: string;
  port: number;
  urls: string[];
  error: string | null;
}

export interface AdminSessionSnapshot {
  session_state: AdminSessionState;
  remaining_seconds: number | null;
  timeout_minutes: number;
  warning_minutes: number;
  action: string;
  autostart_enabled: boolean;
  first_run_complete: boolean;
  session_start_pending: boolean;
  timer_start_timestamp: number | null;
  timer_paused_at: number | null;
  pause_reason: PauseReason | null;
  session_expired: boolean;
  warning_notification_sent: boolean;
}

export interface SessionResponse {
  session: AdminSessionSnapshot;
  warning: string | null;
}

export interface LoginResponse {
  token: string;
  expiresAt: number;
  session: AdminSessionSnapshot;
  adminPanel: AdminPanelInfo;
}

export interface SettingsPayload {
  timeoutMinutes: number;
  warningMinutes: number;
  action: string;
  autostartEnabled: boolean;
}

const SESSION_STORAGE_KEY = "sessionizer-admin-token";

export class AdminApiError extends Error {
  status: number;

  constructor(message: string, status: number) {
    super(message);
    this.name = "AdminApiError";
    this.status = status;
  }
}

function readJsonError(payload: unknown, fallback: string): string {
  if (
    typeof payload === "object" &&
    payload !== null &&
    "error" in payload &&
    typeof payload.error === "string"
  ) {
    return payload.error;
  }

  return fallback;
}

async function request<T>(
  path: string,
  init: RequestInit = {},
  token?: string,
): Promise<T> {
  const headers = new Headers(init.headers);

  if (init.body !== undefined) {
    headers.set("Content-Type", "application/json");
  }

  if (token) {
    headers.set("Authorization", `Bearer ${token}`);
  }

  const response = await fetch(path, {
    ...init,
    headers,
  });

  if (response.status === 204) {
    return undefined as T;
  }

  const payload = (await response.json()) as unknown;

  if (!response.ok) {
    throw new AdminApiError(
      readJsonError(payload, `Request failed with status ${response.status}`),
      response.status,
    );
  }

  return payload as T;
}

export function readStoredToken() {
  return sessionStorage.getItem(SESSION_STORAGE_KEY);
}

export function storeToken(token: string) {
  sessionStorage.setItem(SESSION_STORAGE_KEY, token);
}

export function clearStoredToken() {
  sessionStorage.removeItem(SESSION_STORAGE_KEY);
}

export function login(password: string) {
  return request<LoginResponse>("/api/login", {
    method: "POST",
    body: JSON.stringify({ password }),
  });
}

export function logout(token: string) {
  return request<void>("/api/logout", { method: "POST" }, token);
}

export function getState(token: string) {
  return request<SessionResponse>("/api/state", undefined, token);
}

export function pauseSession(token: string) {
  return request<SessionResponse>("/api/pause", { method: "POST" }, token);
}

export function resumeSession(token: string) {
  return request<SessionResponse>("/api/resume", { method: "POST" }, token);
}

export function unlockSession(token: string) {
  return request<SessionResponse>("/api/unlock", { method: "POST" }, token);
}

export function relockSession(token: string) {
  return request<SessionResponse>("/api/relock", { method: "POST" }, token);
}

export function adjustTime(token: string, deltaMinutes: number) {
  return request<SessionResponse>(
    "/api/adjust-time",
    {
      method: "POST",
      body: JSON.stringify({ deltaMinutes }),
    },
    token,
  );
}

export function updateSettings(token: string, payload: SettingsPayload) {
  return request<SessionResponse>(
    "/api/settings",
    {
      method: "PUT",
      body: JSON.stringify(payload),
    },
    token,
  );
}
