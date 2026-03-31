interface CountdownTimerProps {
  remainingSeconds: number | null;
  totalSeconds: number;
  isWarning: boolean;
  isUrgent: boolean;
}

function formatTime(seconds: number): string {
  const mins = Math.floor(seconds / 60);
  const secs = seconds % 60;
  return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
}

export function CountdownTimer({
  remainingSeconds,
  totalSeconds,
  isWarning,
  isUrgent,
}: CountdownTimerProps) {
  if (remainingSeconds === null) {
    return (
      <div className="text-6xl font-bold text-slate-400">--:--</div>
    );
  }

  const progress = totalSeconds > 0
    ? Math.max(0, Math.min(100, (remainingSeconds / totalSeconds) * 100))
    : 0;

  const themeColor = isUrgent 
    ? "text-red-500" 
    : isWarning 
      ? "text-orange-500" 
      : "text-blue-500";

  const progressColor = isUrgent 
    ? "bg-red-500" 
    : isWarning 
      ? "bg-orange-500" 
      : "bg-blue-500";

  const animationClass = isUrgent 
    ? "animate-pulse-urgent" 
    : isWarning 
      ? "animate-pulse-warning" 
      : "";

  return (
    <div className={`text-center ${animationClass}`}>
      <div className={`text-7xl font-bold mb-4 ${themeColor}`}>
        {formatTime(remainingSeconds)}
      </div>
      <div className="w-full max-w-md mx-auto bg-slate-700 rounded-full h-3 overflow-hidden">
        <div
          className={`h-full ${progressColor} transition-all duration-1000`}
          style={{ width: `${progress}%` }}
        />
      </div>
      <p className="text-slate-400 mt-2 text-sm">Time remaining</p>
    </div>
  );
}
