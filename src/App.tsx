import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { enable as enableAutostart } from "@tauri-apps/plugin-autostart";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import {
  clearTimer,
  executeExpiredAction,
  getConfig,
  getRemainingSeconds,
  markWarningNotificationSent,
  pauseTimer,
  quitApp,
  resumeTimer,
  startTimer,
  type ExpiredActionStatus,
} from "./lib/invoke";
import { FatalErrorScreen } from "./components/FatalErrorScreen";
import { LockScreen } from "./components/LockScreen";
import { PasswordInput } from "./components/PasswordInput";
import { PausedPanel } from "./components/PausedPanel";
import { SettingsPanel } from "./components/SettingsPanel";
import { SetupWizard } from "./components/SetupWizard";
import { UnlockedPanel } from "./components/UnlockedPanel";
import { getWarningNotificationBody } from "./lib/warningNotification";

type View = "loading" | "setup" | "lock" | "paused" | "unlocked" | "fatal";
type PasswordPromptMode = "settings" | "quit" | "relock" | "pause" | null;

const STARTUP_ERROR_MESSAGE =
  "Sessionizer could not load its configuration. The session remains locked until an adult repairs the configuration file.";
const RUNTIME_ERROR_MESSAGE =
  "Sessionizer lost access to its configuration while running. The session remains locked until an adult repairs the configuration file.";
const ACTION_FAILURE_MESSAGE =
  "Time is up. Sessionizer could not complete the configured system action, so the session remains locked.";

function App() {
  const [view, setView] = useState<View>("loading");
  const [showSettings, setShowSettings] = useState(false);
  const [showPasswordPrompt, setShowPasswordPrompt] = useState(false);
  const [passwordPromptMode, setPasswordPromptMode] =
    useState<PasswordPromptMode>(null);
  const [timeoutMinutes, setTimeoutMinutes] = useState(60);
  const [warningMinutes, setWarningMinutes] = useState(5);
  const [runtimeError, setRuntimeError] = useState<string | null>(null);
  const [lockMessage, setLockMessage] = useState<string | null>(null);
  const [sessionExpired, setSessionExpired] = useState(false);

  const viewRef = useRef(view);
  const hasExecutedRef = useRef(false);
  const lastTimerStartRef = useRef<number | null>(null);
  const warningNotificationPendingRef = useRef(false);
  const warningNotificationHandledRef = useRef(false);

  useEffect(() => {
    viewRef.current = view;
  }, [view]);

  const resetTimerState = useCallback(
    (timerStartTimestamp: number | null, expired: boolean = false) => {
      hasExecutedRef.current = expired;
      lastTimerStartRef.current = timerStartTimestamp;
      warningNotificationPendingRef.current = false;
      warningNotificationHandledRef.current = false;
      setSessionExpired(expired);
      setLockMessage(null);
    },
    [],
  );

  const showFatalError = useCallback(
    async (message: string, error: unknown) => {
      console.error(message, error);
      resetTimerState(null, true);
      setRuntimeError(message);
      setShowSettings(false);
      setShowPasswordPrompt(false);
      setPasswordPromptMode(null);
      setView("fatal");

      try {
        await getCurrentWindow().show();
      } catch (showError) {
        console.error("Failed to show fatal error screen:", showError);
      }
    },
    [resetTimerState],
  );

  const syncConfig = useCallback(async () => {
    const config = await getConfig();
    setRuntimeError(null);
    setTimeoutMinutes(config.timeout_minutes);
    setWarningMinutes(config.warning_minutes);
    setSessionExpired(config.session_expired);
    return config;
  }, []);

  const loadRuntimeState = useCallback(async () => {
    try {
      const config = await syncConfig();
      const win = getCurrentWindow();

      if (config.first_run_complete && config.autostart_enabled) {
        void enableAutostart().catch((error) => {
          console.error("Failed to refresh autostart entry:", error);
        });
      }

      if (!config.first_run_complete) {
        resetTimerState(null);
        setView("setup");
        await win.show();
        return;
      }

      const remaining = await getRemainingSeconds();

      if (
        config.timer_start_timestamp !== null &&
        config.timer_paused_at !== null &&
        config.pause_reason === "manual"
      ) {
        resetTimerState(config.timer_start_timestamp);
        setView("paused");
        await win.hide();
        return;
      }

      if (remaining === null) {
        resetTimerState(null);
        setView("unlocked");
        await win.hide();
        return;
      }

      resetTimerState(
        config.timer_start_timestamp,
        Boolean(config.session_expired),
      );
      setView("lock");
      await win.show();
    } catch (error) {
      await showFatalError(STARTUP_ERROR_MESSAGE, error);
    }
  }, [resetTimerState, showFatalError, syncConfig]);

  useEffect(() => {
    void loadRuntimeState();
  }, [loadRuntimeState]);

  const openPasswordPrompt = useCallback(
    (mode: Exclude<PasswordPromptMode, null>) => {
      setPasswordPromptMode(mode);
      setShowPasswordPrompt(true);
    },
    [],
  );

  useEffect(() => {
    const unlistenFns: Array<() => void> = [];
    let disposed = false;

    const setupListeners = async () => {
      unlistenFns.push(
        await listen("show-settings", () => {
          openPasswordPrompt("settings");
        }),
      );

      unlistenFns.push(
        await listen("re-lock", async () => {
          if (viewRef.current === "unlocked") {
            openPasswordPrompt("relock");
            return;
          }

          if (viewRef.current === "lock") {
            const win = getCurrentWindow();
            await win.show();
          }
        }),
      );

      unlistenFns.push(
        await listen("resume-session", async () => {
          try {
            const config = await getConfig();
            if (
              config.pause_reason !== "manual" ||
              config.timer_paused_at === null
            ) {
              return;
            }

            await resumeTimer();
            resetTimerState(null);
            setView("lock");
            const win = getCurrentWindow();
            await win.show();
            await win.setFocus();
          } catch (error) {
            await showFatalError(RUNTIME_ERROR_MESSAGE, error);
          }
        }),
      );

      unlistenFns.push(
        await listen("runtime-state-changed", async () => {
          try {
            await loadRuntimeState();
          } catch (error) {
            await showFatalError(RUNTIME_ERROR_MESSAGE, error);
          }
        }),
      );

      unlistenFns.push(
        await listen("show-about", () => {
          alert(
            "Sessionizer v1.1.0\nA parental screen-time session panel for Windows",
          );
        }),
      );

      unlistenFns.push(
        await listen("quit-app", () => {
          openPasswordPrompt("quit");
        }),
      );

      if (disposed) {
        for (const unlisten of unlistenFns) {
          unlisten();
        }
      }
    };

    void setupListeners();

    return () => {
      disposed = true;
      for (const unlisten of unlistenFns) {
        unlisten();
      }
    };
  }, [loadRuntimeState, openPasswordPrompt, resetTimerState, showFatalError]);

  const handleSetupComplete = useCallback(async () => {
    const config = await syncConfig();
    await startTimer();
    const updatedConfig = await getConfig();
    resetTimerState(updatedConfig.timer_start_timestamp);
    setView("lock");
    setWarningMinutes(config.warning_minutes);
    await getCurrentWindow().show();
  }, [resetTimerState, syncConfig]);

  const handleUnlock = useCallback(async () => {
    await clearTimer();
    resetTimerState(null);
    setView("unlocked");
    await getCurrentWindow().hide();
  }, [resetTimerState]);

  const handlePasswordPromptClose = useCallback(() => {
    setShowPasswordPrompt(false);
    setPasswordPromptMode(null);
  }, []);

  const handlePasswordPromptSuccess = useCallback(async () => {
    const win = getCurrentWindow();

    if (passwordPromptMode === "quit") {
      try {
        await clearTimer();
        await quitApp();
      } catch (error) {
        console.error("Failed to quit app:", error);
      } finally {
        setShowPasswordPrompt(false);
        setPasswordPromptMode(null);
      }
      return;
    }

    if (passwordPromptMode === "relock") {
      try {
        await syncConfig();
        await startTimer();
        const updatedConfig = await getConfig();
        resetTimerState(updatedConfig.timer_start_timestamp);
        setView("lock");
        setShowPasswordPrompt(false);
        setPasswordPromptMode(null);
        await win.show();
      } catch (error) {
        console.error("Failed to re-lock session:", error);
      }
      return;
    }

    if (passwordPromptMode === "pause") {
      try {
        await pauseTimer();
        resetTimerState(null);
        setView("paused");
        setShowPasswordPrompt(false);
        setPasswordPromptMode(null);
        await win.hide();
      } catch (error) {
        console.error("Failed to pause session:", error);
      }
      return;
    }

    setShowPasswordPrompt(false);
    setPasswordPromptMode(null);
    setShowSettings(true);
    await win.show();
  }, [passwordPromptMode, resetTimerState, syncConfig]);

  const handleSettingsClose = useCallback(async () => {
    setShowSettings(false);

    try {
      await syncConfig();
      if (viewRef.current === "unlocked" || viewRef.current === "paused") {
        await getCurrentWindow().hide();
      }
    } catch (error) {
      await showFatalError(RUNTIME_ERROR_MESSAGE, error);
    }
  }, [showFatalError, syncConfig]);

  const handleHideUnlocked = useCallback(async () => {
    await getCurrentWindow().hide();
  }, []);

  useEffect(() => {
    if (view !== "lock") {
      return;
    }

    let isActive = true;

    const checkTimer = async () => {
      try {
        const [remaining, config] = await Promise.all([
          getRemainingSeconds(),
          getConfig(),
        ]);

        if (!isActive) {
          return;
        }

        setSessionExpired(config.session_expired);

        if (config.timer_start_timestamp !== lastTimerStartRef.current) {
          lastTimerStartRef.current = config.timer_start_timestamp;
          hasExecutedRef.current = config.session_expired;
          warningNotificationPendingRef.current = false;
          warningNotificationHandledRef.current = false;
          setLockMessage(null);
        }

        if (remaining === null) {
          resetTimerState(null);
          setView("unlocked");
          await getCurrentWindow().hide();
          return;
        }

        if (config.session_expired) {
          hasExecutedRef.current = true;
          return;
        }

        if (
          remaining > 0 &&
          remaining <= config.warning_minutes * 60 &&
          !config.warning_notification_sent &&
          !warningNotificationPendingRef.current &&
          !warningNotificationHandledRef.current
        ) {
          warningNotificationPendingRef.current = true;

          try {
            let permissionGranted = await isPermissionGranted();
            if (!isActive) {
              return;
            }

            if (!permissionGranted) {
              permissionGranted = (await requestPermission()) === "granted";
              if (!isActive) {
                return;
              }
            }

            if (permissionGranted) {
              sendNotification({
                title: "Session ending soon",
                body: getWarningNotificationBody(remaining),
              });
              try {
                await markWarningNotificationSent();
              } catch (error) {
                console.error(
                  "Failed to mark warning notification as sent:",
                  error,
                );
              }
            }
          } catch (error) {
            console.error("Failed to send warning notification:", error);
          } finally {
            if (isActive) {
              warningNotificationHandledRef.current = true;
              warningNotificationPendingRef.current = false;
            }
          }
        }

        if (remaining > 0 || hasExecutedRef.current) {
          return;
        }

        hasExecutedRef.current = true;

        const status: ExpiredActionStatus = await executeExpiredAction();
        if (!isActive) {
          return;
        }

        if (status === "action_started") {
          setSessionExpired(true);
          setLockMessage(null);
          return;
        }

        if (status === "locked_on_failure") {
          setSessionExpired(true);
          setLockMessage(ACTION_FAILURE_MESSAGE);
          return;
        }

        hasExecutedRef.current = false;
      } catch (error) {
        if (isActive) {
          await showFatalError(RUNTIME_ERROR_MESSAGE, error);
        }
      }
    };

    void checkTimer();
    const interval = window.setInterval(() => {
      void checkTimer();
    }, 1000);

    return () => {
      isActive = false;
      window.clearInterval(interval);
    };
  }, [resetTimerState, showFatalError, view]);

  const passwordPromptTitle =
    passwordPromptMode === "relock"
      ? "Confirm Re-lock"
      : passwordPromptMode === "pause"
        ? "Pause Session"
        : passwordPromptMode === "quit"
          ? "Confirm Quit"
          : "Enter Password";

  const passwordPromptLabel =
    passwordPromptMode === "relock"
      ? "Re-lock Session"
      : passwordPromptMode === "pause"
        ? "Pause Session"
        : passwordPromptMode === "quit"
          ? "Quit App"
          : "Open Settings";

  const passwordPromptLoadingLabel =
    passwordPromptMode === "quit" ? "Quitting..." : "Checking...";

  if (view === "fatal" && runtimeError) {
    return <FatalErrorScreen message={runtimeError} />;
  }

  if (showSettings) {
    return <SettingsPanel onClose={handleSettingsClose} />;
  }

  if (showPasswordPrompt) {
    return (
      <div className="min-h-screen bg-slate-900 flex items-center justify-center p-4">
        <div className="bg-slate-800 rounded-2xl p-8 shadow-2xl max-w-sm w-full">
          <h2 className="text-xl font-bold text-white text-center mb-6">
            {passwordPromptTitle}
          </h2>
          <PasswordInput
            onSuccess={handlePasswordPromptSuccess}
            onCancel={handlePasswordPromptClose}
            submitLabel={passwordPromptLabel}
            loadingLabel={passwordPromptLoadingLabel}
          />
        </div>
      </div>
    );
  }

  if (view === "loading") {
    return (
      <div className="min-h-screen bg-slate-900 flex items-center justify-center">
        <div className="text-white text-xl">Loading...</div>
      </div>
    );
  }

  if (view === "setup") {
    return <SetupWizard onComplete={handleSetupComplete} />;
  }

  if (view === "unlocked") {
    return (
      <UnlockedPanel
        onHide={handleHideUnlocked}
        onOpenSettings={() => openPasswordPrompt("settings")}
        onReLock={() => openPasswordPrompt("relock")}
      />
    );
  }

  if (view === "paused") {
    return (
      <PausedPanel
        onHide={handleHideUnlocked}
        onOpenSettings={() => openPasswordPrompt("settings")}
        timeoutMinutes={timeoutMinutes}
        warningMinutes={warningMinutes}
      />
    );
  }

  return (
    <LockScreen
      onUnlock={handleUnlock}
      onPause={() => openPasswordPrompt("pause")}
      timeoutMinutes={timeoutMinutes}
      warningMinutes={warningMinutes}
      isSessionExpired={sessionExpired}
      statusMessage={lockMessage}
    />
  );
}

export default App;
