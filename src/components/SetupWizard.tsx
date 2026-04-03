import { useState } from "react";
import { setupPassword, getConfig, saveConfig } from "../lib/invoke";
import { enable } from "@tauri-apps/plugin-autostart";

interface SetupWizardProps {
  onComplete: () => void;
}

export function SetupWizard({ onComplete }: SetupWizardProps) {
  const [step, setStep] = useState(1);
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [timeoutMinutes, setTimeoutMinutes] = useState(60);
  const [recoveryKey, setRecoveryKey] = useState("");
  const [savedKey, setSavedKey] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleStep1 = () => {
    setError(null);
    setStep(2);
  };

  const handleStep2 = async () => {
    if (password.length < 4) {
      setError("Password must be at least 4 characters");
      return;
    }
    if (password !== confirmPassword) {
      setError("Passwords do not match");
      return;
    }

    setLoading(true);
    setError(null);
    setWarning(null);

    try {
      const key = await setupPassword(password, timeoutMinutes);

      try {
        await enable();
      } catch {
        setWarning(
          "Setup can continue, but Sessionizer could not enable Start with Windows. You can retry this later in Settings.",
        );
      }

      setRecoveryKey(key);
      setStep(3);
    } catch {
      setError("Failed to set up password");
    } finally {
      setLoading(false);
    }
  };

  const handleStep3 = async () => {
    if (!savedKey) {
      setError("Please confirm you have saved the recovery key");
      return;
    }

    try {
      const config = await getConfig();
      await saveConfig({ ...config, first_run_complete: true });
      onComplete();
    } catch {
      setError("Failed to complete setup");
    }
  };

  return (
    <div className="min-h-screen bg-slate-900 flex items-center justify-center p-4">
      <div className="bg-slate-800 rounded-2xl p-8 shadow-2xl max-w-lg w-full">
        <div className="flex items-center justify-center mb-6">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            className="h-16 w-16 text-blue-500"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"
            />
          </svg>
        </div>

        {step === 1 && (
          <>
            <h1 className="text-3xl font-bold text-white text-center mb-4">
              Welcome to Sessionizer
            </h1>
            <p className="text-slate-400 text-center mb-8">
              Sessionizer helps you manage screen time for your children. Set a
              time limit, and the computer can shut down, restart, or sleep when
              time runs out.
            </p>
            <div className="space-y-4 text-slate-300">
              <div className="flex items-start gap-3">
                <div className="bg-blue-600 rounded-full p-1 mt-0.5">
                  <span className="text-white text-sm">1</span>
                </div>
                <p>Set a password to control access</p>
              </div>
              <div className="flex items-start gap-3">
                <div className="bg-blue-600 rounded-full p-1 mt-0.5">
                  <span className="text-white text-sm">2</span>
                </div>
                <p>Choose how much screen time is allowed</p>
              </div>
              <div className="flex items-start gap-3">
                <div className="bg-blue-600 rounded-full p-1 mt-0.5">
                  <span className="text-white text-sm">3</span>
                </div>
                <p>Save your recovery key in a safe place</p>
              </div>
            </div>
            <button
              type="button"
              onClick={handleStep1}
              className="w-full mt-8 bg-blue-600 hover:bg-blue-700 rounded-lg px-6 py-3 font-semibold transition-colors"
            >
              Get Started
            </button>
          </>
        )}

        {step === 2 && (
          <>
            <h2 className="text-2xl font-bold text-white text-center mb-6">
              Set Up Your Password
            </h2>
            {error && (
              <p className="text-red-500 text-center mb-4 text-sm">{error}</p>
            )}
            {warning && (
              <p className="text-amber-300 text-center mb-4 text-sm">
                {warning}
              </p>
            )}
            <div className="space-y-4">
              <div>
                <label className="block text-slate-400 mb-2 text-sm">
                  Password
                </label>
                <input
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  className="w-full bg-slate-700 border border-slate-600 rounded-lg px-4 py-3 text-white"
                  placeholder="Enter password (min 4 characters)"
                />
              </div>
              <div>
                <label className="block text-slate-400 mb-2 text-sm">
                  Confirm Password
                </label>
                <input
                  type="password"
                  value={confirmPassword}
                  onChange={(e) => setConfirmPassword(e.target.value)}
                  className="w-full bg-slate-700 border border-slate-600 rounded-lg px-4 py-3 text-white"
                  placeholder="Confirm password"
                />
              </div>
              <div>
                <label className="block text-slate-400 mb-2 text-sm">
                  Screen Time Limit: {timeoutMinutes} minutes
                </label>
                <input
                  type="range"
                  min="5"
                  max="180"
                  value={timeoutMinutes}
                  onChange={(e) => setTimeoutMinutes(Number(e.target.value))}
                  className="w-full"
                />
                <div className="flex justify-between text-xs text-slate-500 mt-1">
                  <span>5 min</span>
                  <span>180 min</span>
                </div>
              </div>
            </div>
            <button
              type="button"
              onClick={handleStep2}
              disabled={loading}
              className="w-full mt-6 bg-blue-600 hover:bg-blue-700 disabled:bg-slate-600 disabled:cursor-not-allowed rounded-lg px-6 py-3 font-semibold transition-colors"
            >
              {loading ? "Setting up..." : "Continue"}
            </button>
          </>
        )}

        {step === 3 && (
          <>
            <h2 className="text-2xl font-bold text-white text-center mb-4">
              Recovery Key
            </h2>
            <p className="text-slate-400 text-center mb-6">
              Save this key in a safe place. You'll need it if you forget your
              password.
            </p>
            {error && (
              <p className="text-red-500 text-center mb-4 text-sm">{error}</p>
            )}
            {warning && (
              <p className="text-amber-300 text-center mb-4 text-sm">
                {warning}
              </p>
            )}
            <div className="bg-slate-700 rounded-lg p-4 text-center mb-6">
              <p className="font-mono text-2xl text-blue-400 tracking-wider">
                {recoveryKey}
              </p>
            </div>
            <div className="flex items-center gap-3 mb-6">
              <input
                type="checkbox"
                id="savedKey"
                checked={savedKey}
                onChange={(e) => setSavedKey(e.target.checked)}
                className="w-5 h-5"
              />
              <label htmlFor="savedKey" className="text-slate-300">
                I have saved this key in a safe place
              </label>
            </div>
            <button
              type="button"
              onClick={handleStep3}
              className="w-full bg-blue-600 hover:bg-blue-700 rounded-lg px-6 py-3 font-semibold transition-colors"
            >
              Finish Setup
            </button>
          </>
        )}
      </div>
    </div>
  );
}
