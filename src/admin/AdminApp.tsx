import { startTransition, useEffect, useEffectEvent, useState } from "react";
import {
  AdminApiError,
  adjustTime,
  clearStoredToken,
  getState,
  login,
  logout,
  pauseSession,
  readStoredToken,
  relockSession,
  resumeSession,
  storeToken,
  unlockSession,
  updateSettings,
  type AdminPanelInfo,
  type AdminSessionSnapshot,
} from "./api";
import {
  formatRemainingTime,
  sessionStateCopy,
  sessionStateLabel,
} from "./formatters";

type NoticeTone = "error" | "warning" | "success";

interface Notice {
  tone: NoticeTone;
  message: string;
}

interface SettingsDraft {
  timeoutMinutes: number;
  warningMinutes: number;
  action: string;
  autostartEnabled: boolean;
}

function toSettingsDraft(session: AdminSessionSnapshot): SettingsDraft {
  return {
    timeoutMinutes: session.timeout_minutes,
    warningMinutes: session.warning_minutes,
    action: session.action,
    autostartEnabled: session.autostart_enabled,
  };
}

function AdminApp() {
  const [token, setToken] = useState<string | null>(() => readStoredToken());
  const [password, setPassword] = useState("");
  const [session, setSession] = useState<AdminSessionSnapshot | null>(null);
  const [panelInfo, setPanelInfo] = useState<AdminPanelInfo | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [customDelta, setCustomDelta] = useState(10);
  const [settings, setSettings] = useState<SettingsDraft>({
    timeoutMinutes: 60,
    warningMinutes: 5,
    action: "shutdown",
    autostartEnabled: true,
  });
  const [settingsDirty, setSettingsDirty] = useState(false);

  useEffect(() => {
    if (!session || settingsDirty) {
      return;
    }

    setSettings(toSettingsDraft(session));
  }, [session, settingsDirty]);

  const showNotice = (tone: NoticeTone, message: string) => {
    setNotice({ tone, message });
  };

  const clearSessionState = () => {
    clearStoredToken();
    setToken(null);
    setSession(null);
    setPanelInfo(null);
  };

  const handleError = (error: unknown, fallback: string) => {
    if (error instanceof AdminApiError && error.status === 401) {
      clearSessionState();
      showNotice("warning", "Admin session expired. Sign in again.");
      return;
    }

    showNotice("error", error instanceof Error ? error.message : fallback);
  };

  const applySessionResponse = (
    nextSession: AdminSessionSnapshot,
    warning: string | null,
    success?: string,
  ) => {
    startTransition(() => {
      setSession(nextSession);
    });

    if (warning) {
      showNotice("warning", warning);
      return;
    }

    if (success) {
      showNotice("success", success);
    }
  };

  const refreshState = useEffectEvent(async (activeToken: string) => {
    const response = await getState(activeToken);
    applySessionResponse(response.session, response.warning);
  });

  const handleRefreshError = useEffectEvent((error: unknown) => {
    handleError(error, "Failed to refresh the current session.");
  });

  useEffect(() => {
    if (!token) {
      return;
    }

    void refreshState(token).catch(handleRefreshError);

    const interval = window.setInterval(() => {
      void refreshState(token).catch(handleRefreshError);
    }, 5_000);

    return () => {
      window.clearInterval(interval);
    };
  }, [token]);

  const runAction = async (
    actionName: string,
    operation: (activeToken: string) => Promise<{
      session: AdminSessionSnapshot;
      warning: string | null;
    }>,
    success?: string,
  ) => {
    if (!token) {
      return;
    }

    setBusyAction(actionName);
    setNotice(null);

    try {
      const response = await operation(token);
      applySessionResponse(response.session, response.warning, success);
      if (actionName === "save-settings") {
        setSettingsDirty(false);
      }
    } catch (error) {
      handleError(error, "Remote action failed.");
    } finally {
      setBusyAction(null);
    }
  };

  const handleLogin = async (event: React.FormEvent) => {
    event.preventDefault();
    if (!password.trim()) {
      return;
    }

    setBusyAction("login");
    setNotice(null);

    try {
      const response = await login(password);
      storeToken(response.token);
      setToken(response.token);
      setSession(response.session);
      setPanelInfo(response.adminPanel);
      setPassword("");
      setSettingsDirty(false);
      showNotice("success", "Remote admin unlocked.");
    } catch (error) {
      handleError(error, "Could not sign in to the admin panel.");
    } finally {
      setBusyAction(null);
    }
  };

  const handleLogout = async () => {
    if (!token) {
      return;
    }

    setBusyAction("logout");
    setNotice(null);

    try {
      await logout(token);
      clearSessionState();
      showNotice("success", "Signed out.");
    } catch (error) {
      handleError(error, "Could not sign out cleanly.");
    } finally {
      setBusyAction(null);
    }
  };

  if (!token || !session) {
    return (
      <main className="admin-shell">
        <section className="admin-card admin-login">
          <p className="admin-eyebrow">Field Console</p>
          <h1 className="admin-title">Sessionizer Remote Admin</h1>
          <p className="admin-copy">
            Use the same password as the desktop app. This panel is intended for
            your local network, so keep the app running on the computer you want
            to control.
          </p>

          <div className="admin-meta">
            <span>Origin</span>
            <strong>{window.location.origin}</strong>
          </div>

          {notice && (
            <div className="admin-notice" data-tone={notice.tone}>
              {notice.message}
            </div>
          )}

          <form className="mt-6 grid gap-4" onSubmit={handleLogin}>
            <label className="admin-field">
              <span>Password</span>
              <input
                type="password"
                className="admin-input"
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                placeholder="Enter the app password"
                autoComplete="current-password"
              />
            </label>

            <button
              type="submit"
              className="admin-button admin-button--accent"
              disabled={busyAction === "login" || !password.trim()}
            >
              {busyAction === "login" ? "Checking..." : "Unlock Remote Admin"}
            </button>
          </form>
        </section>
      </main>
    );
  }

  const canPause = session.session_state === "locked";
  const canResume = session.session_state === "paused";
  const canAdjust =
    session.session_state === "locked" || session.session_state === "paused";
  const canUnlock =
    session.session_state !== "setup" && session.session_state !== "unlocked";
  const canRelock = session.session_state !== "setup";

  return (
    <main className="admin-shell">
      <div className="admin-stack">
        <header className="admin-card">
          <div className="flex items-start justify-between gap-4">
            <div>
              <p className="admin-eyebrow">Signal Board</p>
              <h1 className="admin-title">Remote Session Control</h1>
              <p className="admin-copy">{sessionStateCopy(session)}</p>
            </div>
            <button
              type="button"
              className="admin-button admin-button--ghost"
              onClick={handleLogout}
              disabled={busyAction === "logout"}
            >
              {busyAction === "logout" ? "Signing out..." : "Sign out"}
            </button>
          </div>

          <div className="mt-6 grid gap-4 md:grid-cols-[1.3fr_0.7fr]">
            <div className="admin-display-panel">
              <span className="admin-chip" data-state={session.session_state}>
                {sessionStateLabel(session)}
              </span>
              <div className="admin-display">
                {formatRemainingTime(session.remaining_seconds)}
              </div>
              <p className="admin-copy">
                {panelInfo?.running
                  ? `Serving from ${window.location.host}`
                  : "Remote server status is not available."}
              </p>
            </div>

            <div className="grid gap-3">
              <div className="admin-stat">
                <span>Limit</span>
                <strong>{session.timeout_minutes} min</strong>
              </div>
              <div className="admin-stat">
                <span>Warning</span>
                <strong>{session.warning_minutes} min</strong>
              </div>
              <div className="admin-stat">
                <span>Action</span>
                <strong className="capitalize">{session.action}</strong>
              </div>
            </div>
          </div>
        </header>

        {notice && (
          <div className="admin-notice" data-tone={notice.tone}>
            {notice.message}
          </div>
        )}

        <section className="admin-card">
          <div className="admin-section-head">
            <div>
              <p className="admin-eyebrow">Quick Controls</p>
              <h2 className="admin-section-title">Immediate actions</h2>
            </div>
            <button
              type="button"
              className="admin-button admin-button--ghost"
              onClick={() =>
                void runAction("refresh", (activeToken) =>
                  getState(activeToken),
                )
              }
              disabled={busyAction === "refresh"}
            >
              {busyAction === "refresh" ? "Refreshing..." : "Refresh"}
            </button>
          </div>

          <div className="grid gap-3 sm:grid-cols-2">
            <button
              type="button"
              className="admin-button admin-button--accent"
              onClick={() =>
                void runAction("pause", (activeToken) =>
                  pauseSession(activeToken),
                )
              }
              disabled={busyAction !== null || !canPause}
            >
              Pause Timer
            </button>
            <button
              type="button"
              className="admin-button admin-button--accent"
              onClick={() =>
                void runAction("resume", (activeToken) =>
                  resumeSession(activeToken),
                )
              }
              disabled={busyAction !== null || !canResume}
            >
              Resume Timer
            </button>
            <button
              type="button"
              className="admin-button"
              onClick={() =>
                void runAction("unlock", (activeToken) =>
                  unlockSession(activeToken),
                )
              }
              disabled={busyAction !== null || !canUnlock}
            >
              Unlock Session
            </button>
            <button
              type="button"
              className="admin-button"
              onClick={() =>
                void runAction(
                  "relock",
                  (activeToken) => relockSession(activeToken),
                  "Fresh session started.",
                )
              }
              disabled={busyAction !== null || !canRelock}
            >
              Fresh Lock
            </button>
          </div>
        </section>

        <section className="admin-card">
          <div className="admin-section-head">
            <div>
              <p className="admin-eyebrow">Time Shift</p>
              <h2 className="admin-section-title">Add or subtract minutes</h2>
            </div>
          </div>

          <div className="grid gap-3 sm:grid-cols-4">
            {[-15, -5, 5, 15].map((delta) => (
              <button
                key={delta}
                type="button"
                className="admin-button"
                onClick={() =>
                  void runAction("adjust", (activeToken) =>
                    adjustTime(activeToken, delta),
                  )
                }
                disabled={busyAction !== null || !canAdjust}
              >
                {delta > 0 ? `+${delta}` : delta} min
              </button>
            ))}
          </div>

          <div className="mt-4 grid gap-3 sm:grid-cols-[1fr_1fr_1fr]">
            <label className="admin-field">
              <span>Custom minutes</span>
              <input
                type="number"
                className="admin-input"
                min="1"
                max="180"
                value={customDelta}
                onChange={(event) =>
                  setCustomDelta(Math.max(1, Number(event.target.value) || 1))
                }
              />
            </label>
            <button
              type="button"
              className="admin-button"
              onClick={() =>
                void runAction("subtract-custom", (activeToken) =>
                  adjustTime(activeToken, -customDelta),
                )
              }
              disabled={busyAction !== null || !canAdjust}
            >
              Subtract
            </button>
            <button
              type="button"
              className="admin-button admin-button--accent"
              onClick={() =>
                void runAction("add-custom", (activeToken) =>
                  adjustTime(activeToken, customDelta),
                )
              }
              disabled={busyAction !== null || !canAdjust}
            >
              Add
            </button>
          </div>
        </section>

        <section className="admin-card">
          <div className="admin-section-head">
            <div>
              <p className="admin-eyebrow">Settings</p>
              <h2 className="admin-section-title">Runtime configuration</h2>
            </div>
          </div>

          <div className="grid gap-4">
            <label className="admin-field">
              <span>Screen time limit</span>
              <input
                type="number"
                min="5"
                max="180"
                className="admin-input"
                value={settings.timeoutMinutes}
                onChange={(event) => {
                  setSettingsDirty(true);
                  setSettings((current) => ({
                    ...current,
                    timeoutMinutes: Number(event.target.value) || 5,
                  }));
                }}
              />
            </label>

            <label className="admin-field">
              <span>Warning window</span>
              <input
                type="number"
                min="1"
                max="30"
                className="admin-input"
                value={settings.warningMinutes}
                onChange={(event) => {
                  setSettingsDirty(true);
                  setSettings((current) => ({
                    ...current,
                    warningMinutes: Number(event.target.value) || 1,
                  }));
                }}
              />
            </label>

            <label className="admin-field">
              <span>Expiry action</span>
              <select
                className="admin-select"
                value={settings.action}
                onChange={(event) => {
                  setSettingsDirty(true);
                  setSettings((current) => ({
                    ...current,
                    action: event.target.value,
                  }));
                }}
              >
                <option value="shutdown">Shutdown</option>
                <option value="restart">Restart</option>
                <option value="sleep">Sleep</option>
              </select>
            </label>

            <label className="admin-toggle">
              <input
                type="checkbox"
                checked={settings.autostartEnabled}
                onChange={(event) => {
                  setSettingsDirty(true);
                  setSettings((current) => ({
                    ...current,
                    autostartEnabled: event.target.checked,
                  }));
                }}
              />
              <span>Start with Windows</span>
            </label>

            <button
              type="button"
              className="admin-button admin-button--accent"
              onClick={() =>
                void runAction(
                  "save-settings",
                  (activeToken) =>
                    updateSettings(activeToken, {
                      timeoutMinutes: settings.timeoutMinutes,
                      warningMinutes: settings.warningMinutes,
                      action: settings.action,
                      autostartEnabled: settings.autostartEnabled,
                    }),
                  "Settings saved.",
                )
              }
              disabled={busyAction !== null || !settingsDirty}
            >
              Save Settings
            </button>
          </div>
        </section>
      </div>
    </main>
  );
}

export default AdminApp;
