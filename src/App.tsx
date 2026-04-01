import { useState, useEffect, useCallback, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  isPermissionGranted,
  requestPermission,
  sendNotification,
} from "@tauri-apps/plugin-notification";
import {
  startTimer,
  clearTimer,
  pauseTimer,
  resumeTimer,
  quitApp,
  getRemainingSeconds,
  executeShutdown,
  getConfig,
  markWarningNotificationSent,
} from "./lib/invoke";
import { LockScreen } from "./components/LockScreen";
import { SetupWizard } from "./components/SetupWizard";
import { SettingsPanel } from "./components/SettingsPanel";
import { PasswordInput } from "./components/PasswordInput";
import { UnlockedPanel } from "./components/UnlockedPanel";
import { PausedPanel } from "./components/PausedPanel";

type View = "loading" | "setup" | "lock" | "paused" | "unlocked";
type PasswordPromptMode = "settings" | "quit" | "relock" | "pause" | null;

function getWarningBody(remainingSeconds: number): string {
  if (remainingSeconds <= 60) {
    return "Less than a minute remaining";
  }

  const remainingMinutes = Math.ceil(remainingSeconds / 60);
  return `${remainingMinutes} minute${remainingMinutes === 1 ? "" : "s"} remaining`;
}

function App() {
  const [view, setView] = useState<View>("loading");
  const [showSettings, setShowSettings] = useState(false);
  const [showPasswordPrompt, setShowPasswordPrompt] = useState(false);
  const [passwordPromptMode, setPasswordPromptMode] =
    useState<PasswordPromptMode>(null);
  const [warningMinutes, setWarningMinutes] = useState(5);

  const viewRef = useRef(view);
  const hasExecutedRef = useRef(false);
  const lastTimerStartRef = useRef<number | null>(null);
  const warningNotificationPendingRef = useRef(false);
  const warningNotificationHandledRef = useRef(false);

  useEffect(() => {
    viewRef.current = view;
  }, [view]);

  const syncConfig = useCallback(async () => {
    const config = await getConfig();
    setWarningMinutes(config.warning_minutes);
    return config;
  }, []);

  const loadRuntimeState = useCallback(async () => {
    try {
      const config = await syncConfig();
      const win = getCurrentWindow();

      if (!config.first_run_complete) {
        hasExecutedRef.current = false;
        lastTimerStartRef.current = null;
        warningNotificationPendingRef.current = false;
        warningNotificationHandledRef.current = false;
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
        hasExecutedRef.current = false;
        lastTimerStartRef.current = config.timer_start_timestamp;
        warningNotificationPendingRef.current = false;
        warningNotificationHandledRef.current = false;
        setView("paused");
        await win.hide();
        return;
      }

      if (remaining === null) {
        hasExecutedRef.current = false;
        lastTimerStartRef.current = null;
        warningNotificationPendingRef.current = false;
        warningNotificationHandledRef.current = false;
        setView("unlocked");
        await win.hide();
      } else {
        lastTimerStartRef.current = config.timer_start_timestamp;
        warningNotificationPendingRef.current = false;
        warningNotificationHandledRef.current = false;
        setView("lock");
        await win.show();
      }
    } catch (e) {
      console.error("Failed to initialize:", e);
    }
  }, [syncConfig]);

  useEffect(() => {
    void loadRuntimeState();
  }, [loadRuntimeState]);

  const openPasswordPrompt = useCallback((mode: Exclude<PasswordPromptMode, null>) => {
    setPasswordPromptMode(mode);
    setShowPasswordPrompt(true);
  }, []);

  useEffect(() => {
    const unlistenFns: Array<() => void> = [];
    let disposed = false;

    const setupListeners = async () => {
      unlistenFns.push(
        await listen("show-settings", () => {
          openPasswordPrompt("settings");
        })
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
        })
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
            hasExecutedRef.current = false;
            warningNotificationPendingRef.current = false;
            warningNotificationHandledRef.current = false;
            setView("lock");
            const win = getCurrentWindow();
            await win.show();
            await win.setFocus();
          } catch (e) {
            console.error("Failed to resume paused session:", e);
          }
        })
      );

      unlistenFns.push(
        await listen("show-about", () => {
          alert("Sessionizer v1.0.0\nA parental screen-time session panel for Windows");
        })
      );

      unlistenFns.push(
        await listen("quit-app", () => {
          openPasswordPrompt("quit");
        })
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
  }, [openPasswordPrompt]);

  const handleSetupComplete = useCallback(async () => {
    const config = await syncConfig();
    hasExecutedRef.current = false;
    lastTimerStartRef.current = null;
    warningNotificationPendingRef.current = false;
    warningNotificationHandledRef.current = false;
    setView("lock");
    setWarningMinutes(config.warning_minutes);
    await startTimer();
    await getCurrentWindow().show();
  }, [syncConfig]);

  const handleUnlock = useCallback(async () => {
    await clearTimer();
    hasExecutedRef.current = false;
    lastTimerStartRef.current = null;
    warningNotificationPendingRef.current = false;
    warningNotificationHandledRef.current = false;
    setView("unlocked");
    await getCurrentWindow().hide();
  }, []);

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
      } catch (e) {
        console.error("Failed to quit app:", e);
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
        hasExecutedRef.current = false;
        lastTimerStartRef.current = null;
        warningNotificationPendingRef.current = false;
        warningNotificationHandledRef.current = false;
        setView("lock");
        setShowPasswordPrompt(false);
        setPasswordPromptMode(null);
        await win.show();
      } catch (e) {
        console.error("Failed to re-lock session:", e);
      }
      return;
    }

    if (passwordPromptMode === "pause") {
      try {
        await pauseTimer();
        hasExecutedRef.current = false;
        warningNotificationPendingRef.current = false;
        warningNotificationHandledRef.current = false;
        setView("paused");
        setShowPasswordPrompt(false);
        setPasswordPromptMode(null);
        await win.hide();
      } catch (e) {
        console.error("Failed to pause session:", e);
      }
      return;
    }

    setShowPasswordPrompt(false);
    setPasswordPromptMode(null);
    setShowSettings(true);
    await win.show();
  }, [passwordPromptMode, syncConfig]);

  const handleSettingsClose = useCallback(async () => {
    setShowSettings(false);

    try {
      await syncConfig();
      if (viewRef.current === "unlocked" || viewRef.current === "paused") {
        await getCurrentWindow().hide();
      }
    } catch (e) {
      console.error("Failed to refresh settings:", e);
    }
  }, [syncConfig]);

  const handleHideUnlocked = useCallback(async () => {
    await getCurrentWindow().hide();
  }, []);

  useEffect(() => {
    if (view !== "lock") {
      return;
    }

    const checkTimer = async () => {
      try {
        const [remaining, config] = await Promise.all([
          getRemainingSeconds(),
          getConfig(),
        ]);

        if (config.timer_start_timestamp !== lastTimerStartRef.current) {
          lastTimerStartRef.current = config.timer_start_timestamp;
          hasExecutedRef.current = false;
          warningNotificationPendingRef.current = false;
          warningNotificationHandledRef.current = false;
        }

        if (remaining === null) {
          lastTimerStartRef.current = null;
          hasExecutedRef.current = false;
          warningNotificationPendingRef.current = false;
          warningNotificationHandledRef.current = false;
          setView("unlocked");
          await getCurrentWindow().hide();
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
            if (!permissionGranted) {
              permissionGranted = (await requestPermission()) === "granted";
            }

            if (permissionGranted) {
              sendNotification({
                title: "Session ending soon",
                body: getWarningBody(remaining),
              });
              try {
                await markWarningNotificationSent();
              } catch (e) {
                console.error("Failed to mark warning notification as sent:", e);
              }
            }
          } catch (e) {
            console.error("Failed to send warning notification:", e);
          } finally {
            warningNotificationHandledRef.current = true;
            warningNotificationPendingRef.current = false;
          }
        }

        if (remaining > 0 || hasExecutedRef.current) {
          return;
        }

        hasExecutedRef.current = true;
        await clearTimer();
        setView("unlocked");
        await getCurrentWindow().hide();
        await executeShutdown(config.action);
      } catch (e) {
        console.error("Failed to check timer:", e);
      }
    };

    void checkTimer();
    const interval = window.setInterval(() => {
      void checkTimer();
    }, 1000);

    return () => window.clearInterval(interval);
  }, [view]);

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
        warningMinutes={warningMinutes}
      />
    );
  }

  return (
    <LockScreen
      onUnlock={handleUnlock}
      onPause={() => openPasswordPrompt("pause")}
      warningMinutes={warningMinutes}
    />
  );
}

export default App;
