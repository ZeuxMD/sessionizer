import { useState } from "react";
import { verifyRecoveryKey, resetPasswordWithRecovery } from "../lib/invoke";

interface RecoveryPromptProps {
  onRecovered: () => void;
  onCancel: () => void;
}

export function RecoveryPrompt({ onRecovered, onCancel }: RecoveryPromptProps) {
  const [step, setStep] = useState<"verify" | "reset">("verify");
  const [key, setKey] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleVerify = async (e: React.FormEvent) => {
    e.preventDefault();
    if (key.trim().length !== 16) {
      setError("Recovery key must be 16 characters");
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const valid = await verifyRecoveryKey(key);
      if (valid) {
        setStep("reset");
        setError(null);
      } else {
        setError("Invalid recovery key");
        setKey("");
      }
    } catch {
      setError("Failed to verify recovery key");
    } finally {
      setLoading(false);
    }
  };

  const handleReset = async (e: React.FormEvent) => {
    e.preventDefault();

    if (newPassword.length < 4) {
      setError("New password must be at least 4 characters");
      return;
    }

    if (newPassword !== confirmPassword) {
      setError("New passwords do not match");
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const reset = await resetPasswordWithRecovery(key, newPassword);
      if (reset) {
        onRecovered();
      } else {
        setError("Invalid recovery key");
        setStep("verify");
        setNewPassword("");
        setConfirmPassword("");
      }
    } catch {
      setError("Failed to reset password");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/70 backdrop-blur-sm flex items-center justify-center z-50">
      <div className="bg-slate-800 rounded-2xl p-8 shadow-2xl max-w-md w-full mx-4">
        {step === "verify" ? (
          <>
            <h2 className="text-2xl font-bold text-white mb-4">Recovery Key</h2>
            <p className="text-slate-400 mb-6">
              Enter your 16-character recovery key to reset the password.
            </p>
            <form onSubmit={handleVerify}>
              <input
                type="text"
                value={key}
                onChange={(e) => setKey(e.target.value.toUpperCase())}
                placeholder="Enter recovery key"
                maxLength={16}
                className="w-full bg-slate-700 border border-slate-600 rounded-lg px-4 py-3 text-white font-mono uppercase"
                autoFocus
              />
              {error && <p className="text-red-500 mt-2 text-sm">{error}</p>}
              <div className="flex gap-4 mt-6">
                <button
                  type="button"
                  onClick={onCancel}
                  className="flex-1 bg-slate-600 hover:bg-slate-700 rounded-lg px-6 py-3 font-semibold transition-colors"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={loading || key.length !== 16}
                  className="flex-1 bg-blue-600 hover:bg-blue-700 disabled:bg-slate-600 disabled:cursor-not-allowed rounded-lg px-6 py-3 font-semibold transition-colors"
                >
                  {loading ? "Verifying..." : "Continue"}
                </button>
              </div>
            </form>
          </>
        ) : (
          <>
            <h2 className="text-2xl font-bold text-white mb-4">
              Reset Password
            </h2>
            <p className="text-slate-400 mb-6">
              Set a new password for Sessionizer.
            </p>
            <form onSubmit={handleReset} className="space-y-4">
              <input
                type="password"
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
                placeholder="New password"
                className="w-full bg-slate-700 border border-slate-600 rounded-lg px-4 py-3 text-white"
                autoFocus
              />
              <input
                type="password"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                placeholder="Confirm new password"
                className="w-full bg-slate-700 border border-slate-600 rounded-lg px-4 py-3 text-white"
              />
              {error && <p className="text-red-500 text-sm">{error}</p>}
              <div className="flex gap-4 pt-2">
                <button
                  type="button"
                  onClick={onCancel}
                  className="flex-1 bg-slate-600 hover:bg-slate-700 rounded-lg px-6 py-3 font-semibold transition-colors"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={loading || !newPassword || !confirmPassword}
                  className="flex-1 bg-blue-600 hover:bg-blue-700 disabled:bg-slate-600 disabled:cursor-not-allowed rounded-lg px-6 py-3 font-semibold transition-colors"
                >
                  {loading ? "Saving..." : "Reset Password"}
                </button>
              </div>
            </form>
          </>
        )}
      </div>
    </div>
  );
}
