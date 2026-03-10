import { useState, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isFirstRun, startTimer, clearTimer, quitApp } from "./lib/invoke";
import { LockScreen } from "./components/LockScreen";
import { SetupWizard } from "./components/SetupWizard";
import { SettingsPanel } from "./components/SettingsPanel";
import { PasswordInput } from "./components/PasswordInput";

type View = "loading" | "setup" | "lock" | "unlocked";

function App() {
  const [view, setView] = useState<View>("loading");
  const [showSettings, setShowSettings] = useState(false);
  const [showPasswordPrompt, setShowPasswordPrompt] = useState(false);
  const [passwordPromptMode, setPasswordPromptMode] = useState<"settings" | "quit" | null>(null);
  const [warningMinutes, setWarningMinutes] = useState(5);

  useEffect(() => {
    const init = async () => {
      try {
        const firstRun = await isFirstRun();
        if (firstRun) {
          setView("setup");
        } else {
          setView("lock");
          await startTimer();
        }
      } catch (e) {
        console.error("Failed to initialize:", e);
      }
    };
    init();
  }, []);

  useEffect(() => {
    const setupListeners = async () => {
      await listen("show-settings", () => {
        setPasswordPromptMode("settings");
        setShowPasswordPrompt(true);
      });

      await listen("re-lock", async () => {
        await startTimer();
        setView("lock");
        const win = getCurrentWindow();
        await win.setFullscreen(true);
        await win.show();
      });

      await listen("show-about", () => {
        alert("Sessionizer v1.0.0\nA parental screen-time limiter for Windows");
      });

      await listen("quit-app", () => {
        setPasswordPromptMode("quit");
        setShowPasswordPrompt(true);
      });
    };

    setupListeners();
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (view !== "lock") return;
      
      if (e.key === "F4" && e.altKey) {
        e.preventDefault();
      }
      if (e.key === "w" && e.ctrlKey) {
        e.preventDefault();
      }
      if (e.key === "Tab" && e.altKey) {
        e.preventDefault();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [view]);

  const handleSetupComplete = useCallback(async () => {
    setView("lock");
    await startTimer();
  }, []);

  const handleUnlock = useCallback(async () => {
    await clearTimer();
    setView("unlocked");
    const win = getCurrentWindow();
    await win.setFullscreen(false);
    await win.hide();
  }, []);

  const handleSettingsPasswordSuccess = useCallback(async () => {
    if (passwordPromptMode === "quit") {
      try{
        await quitApp();
      } catch (e) {
        console.error("Failed to quit app:", e);
        setShowPasswordPrompt(false);
        setPasswordPromptMode(null);
      }
    } else {
      setShowPasswordPrompt(false);
      setPasswordPromptMode(null);
      setShowSettings(true);
    }
  }, [passwordPromptMode]);

  const handleSettingsClose = useCallback(async () => {
    setShowSettings(false);
    setView("lock");
  }, []);

  const handleReLock = useCallback(async () => {
    await startTimer();
    setView("lock");
    const win = getCurrentWindow();
    await win.setFullscreen(true);
    await win.show();
  }, []);

  if (showSettings) {
    return <SettingsPanel onClose={handleSettingsClose} />;
  }

  if (showPasswordPrompt) {
    return (
      <div className="min-h-screen bg-slate-900 flex items-center justify-center p-4">
        <div className="bg-slate-800 rounded-2xl p-8 shadow-2xl max-w-sm w-full">
          <h2 className="text-xl font-bold text-white text-center mb-6">Enter Password</h2>
          <PasswordInput onSuccess={handleSettingsPasswordSuccess} />
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

  return (
    <LockScreen
      onUnlock={handleUnlock}
      warningMinutes={warningMinutes}
    />
  );
}

export default App;
