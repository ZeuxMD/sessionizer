import { useState } from "react";
import { verifyRecoveryKey } from "../lib/invoke";

interface RecoveryPromptProps {
  onRecovered: () => void;
  onCancel: () => void;
}

export function RecoveryPrompt({ onRecovered, onCancel }: RecoveryPromptProps) {
  const [key, setKey] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
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
        onRecovered();
      } else {
        setError("Invalid recovery key");
        setKey("");
        setTimeout(() => setError(null), 3000);
      }
    } catch (e) {
      setError("Failed to verify recovery key");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 bg-black/70 backdrop-blur-sm flex items-center justify-center z-50">
      <div className="bg-slate-800 rounded-2xl p-8 shadow-2xl max-w-md w-full mx-4">
        <h2 className="text-2xl font-bold text-white mb-4">Recovery Key</h2>
        <p className="text-slate-400 mb-6">
          Enter your 16-character recovery key to reset your password.
        </p>
        <form onSubmit={handleSubmit}>
          <input
            type="text"
            value={key}
            onChange={(e) => setKey(e.target.value.toUpperCase())}
            placeholder="Enter recovery key"
            maxLength={16}
            className="w-full bg-slate-700 border border-slate-600 rounded-lg px-4 py-3 text-white font-mono uppercase"
            autoFocus
          />
          {error && (
            <p className="text-red-500 mt-2 text-sm">{error}</p>
          )}
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
              {loading ? "Verifying..." : "Submit"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
