import { CountdownTimer } from "./CountdownTimer";
import { useCountdown } from "../hooks/useCountdown";

interface PausedPanelProps {
  onHide: () => void;
  onOpenSettings: () => void;
  warningMinutes: number;
}

export function PausedPanel({
  onHide,
  onOpenSettings,
  warningMinutes,
}: PausedPanelProps) {
  const { remainingSeconds, totalSeconds, isWarning, isUrgent } =
    useCountdown(warningMinutes);

  return (
    <div className="min-h-screen bg-slate-900 flex items-center justify-center p-4">
      <div className="bg-slate-800 rounded-2xl p-8 shadow-2xl max-w-lg w-full text-center">
        <div className="flex items-center justify-center mb-6">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            className="h-16 w-16 text-amber-400"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M10 9v6m4-6v6m5-3a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>
        </div>

        <h1 className="text-3xl font-bold text-white mb-2">Session Paused</h1>
        <p className="text-slate-400 mb-8">
          The child session is paused. Use the tray menu to resume it later.
        </p>

        <CountdownTimer
          remainingSeconds={remainingSeconds}
          totalSeconds={totalSeconds}
          isWarning={isWarning}
          isUrgent={isUrgent}
        />

        <div className="grid gap-3 mt-8">
          <button
            onClick={onOpenSettings}
            className="w-full bg-blue-600 hover:bg-blue-700 rounded-lg px-6 py-3 font-semibold transition-colors"
          >
            Open Settings
          </button>
          <button
            onClick={onHide}
            className="w-full bg-slate-800 hover:bg-slate-700 border border-slate-600 rounded-lg px-6 py-3 font-semibold transition-colors text-white"
          >
            Hide Window
          </button>
        </div>
      </div>
    </div>
  );
}
