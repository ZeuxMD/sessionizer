import { useState, useEffect } from "react";
import { getConfig, saveConfig, changePassword, AppConfig } from "../lib/invoke";

interface SettingsPanelProps {
  onClose: () => void;
}

export function SettingsPanel({ onClose }: SettingsPanelProps) {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [timeoutMinutes, setTimeoutMinutes] = useState(60);
  const [warningMinutes, setWarningMinutes] = useState(5);
  const [action, setAction] = useState("shutdown");
  const [autostartEnabled, setAutostartEnabled] = useState(true);
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [confirmNewPassword, setConfirmNewPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    const fetchConfig = async () => {
      try {
        const cfg = await getConfig();
        setConfig(cfg);
        setTimeoutMinutes(cfg.timeout_minutes);
        setWarningMinutes(cfg.warning_minutes);
        setAction(cfg.action);
        setAutostartEnabled(cfg.autostart_enabled);
      } catch (e) {
        setError("Failed to load settings");
      }
    };
    fetchConfig();
  }, []);

  const handleSave = async () => {
    if (!config) return;

    setLoading(true);
    setError(null);
    setSuccess(null);

    try {
      let cfg: AppConfig = {
        ...config,
        timeout_minutes: timeoutMinutes,
        warning_minutes: warningMinutes,
        action,
        autostart_enabled: autostartEnabled,
      };

      if (newPassword) {
        if (newPassword.length < 4) {
          setError("New password must be at least 4 characters");
          setLoading(false);
          return;
        }
        if (newPassword !== confirmNewPassword) {
          setError("New passwords do not match");
          setLoading(false);
          return;
        }

        const changed = await changePassword(currentPassword, newPassword);
        if (!changed) {
          setError("Current password is incorrect");
          setLoading(false);
          return;
        }
        setNewPassword("");
        setConfirmNewPassword("");
        setCurrentPassword("");
      }

      await saveConfig(cfg);
      setSuccess("Settings saved successfully");
      setTimeout(() => {
        onClose();
      }, 1000);
    } catch (e) {
      setError("Failed to save settings");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-slate-900 flex items-center justify-center p-4">
      <div className="bg-slate-800 rounded-2xl p-8 shadow-2xl max-w-lg w-full">
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-2xl font-bold text-white">Settings</h2>
          <button
            onClick={onClose}
            className="text-slate-400 hover:text-white"
          >
            <svg xmlns="http://www.w3.org/2000/svg" className="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {error && (
          <p className="text-red-500 text-center mb-4 text-sm bg-red-500/10 p-2 rounded">{error}</p>
        )}
        {success && (
          <p className="text-green-500 text-center mb-4 text-sm bg-green-500/10 p-2 rounded">{success}</p>
        )}

        <div className="space-y-6">
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

          <div>
            <label className="block text-slate-400 mb-2 text-sm">
              Warning Time: {warningMinutes} minutes before end
            </label>
            <input
              type="range"
              min="1"
              max="30"
              value={warningMinutes}
              onChange={(e) => setWarningMinutes(Number(e.target.value))}
              className="w-full"
            />
            <div className="flex justify-between text-xs text-slate-500 mt-1">
              <span>1 min</span>
              <span>30 min</span>
            </div>
          </div>

          <div>
            <label className="block text-slate-400 mb-2 text-sm">When Time Runs Out:</label>
            <div className="flex gap-4">
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="radio"
                  name="action"
                  value="shutdown"
                  checked={action === "shutdown"}
                  onChange={(e) => setAction(e.target.value)}
                  className="w-4 h-4"
                />
                <span className="text-white">Shutdown</span>
              </label>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="radio"
                  name="action"
                  value="restart"
                  checked={action === "restart"}
                  onChange={(e) => setAction(e.target.value)}
                  className="w-4 h-4"
                />
                <span className="text-white">Restart</span>
              </label>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="radio"
                  name="action"
                  value="sleep"
                  checked={action === "sleep"}
                  onChange={(e) => setAction(e.target.value)}
                  className="w-4 h-4"
                />
                <span className="text-white">Sleep</span>
              </label>
            </div>
          </div>

          <div className="border-t border-slate-700 pt-6">
            <h3 className="text-lg font-semibold text-white mb-4">Change Password</h3>
            <div className="space-y-4">
              <div>
                <label className="block text-slate-400 mb-2 text-sm">Current Password</label>
                <input
                  type="password"
                  value={currentPassword}
                  onChange={(e) => setCurrentPassword(e.target.value)}
                  className="w-full bg-slate-700 border border-slate-600 rounded-lg px-4 py-3 text-white"
                  placeholder="Enter current password"
                />
              </div>
              <div>
                <label className="block text-slate-400 mb-2 text-sm">New Password</label>
                <input
                  type="password"
                  value={newPassword}
                  onChange={(e) => setNewPassword(e.target.value)}
                  className="w-full bg-slate-700 border border-slate-600 rounded-lg px-4 py-3 text-white"
                  placeholder="Enter new password"
                />
              </div>
              <div>
                <label className="block text-slate-400 mb-2 text-sm">Confirm New Password</label>
                <input
                  type="password"
                  value={confirmNewPassword}
                  onChange={(e) => setConfirmNewPassword(e.target.value)}
                  className="w-full bg-slate-700 border border-slate-600 rounded-lg px-4 py-3 text-white"
                  placeholder="Confirm new password"
                />
              </div>
            </div>
          </div>

          <div className="border-t border-slate-700 pt-6">
            <label className="flex items-center gap-3 cursor-pointer">
              <input
                type="checkbox"
                checked={autostartEnabled}
                onChange={(e) => setAutostartEnabled(e.target.checked)}
                className="w-5 h-5"
              />
              <span className="text-white">Start with Windows</span>
            </label>
          </div>
        </div>

        <button
          onClick={handleSave}
          disabled={loading}
          className="w-full mt-8 bg-blue-600 hover:bg-blue-700 disabled:bg-slate-600 disabled:cursor-not-allowed rounded-lg px-6 py-3 font-semibold transition-colors"
        >
          {loading ? "Saving..." : "Save Settings"}
        </button>
      </div>
    </div>
  );
}
