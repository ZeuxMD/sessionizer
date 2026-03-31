interface WarningBannerProps {
  isWarning: boolean;
  isUrgent: boolean;
}

export function WarningBanner({ isWarning, isUrgent }: WarningBannerProps) {
  if (!isWarning && !isUrgent) return null;

  if (isUrgent) {
    return (
      <div className="w-full max-w-md mx-auto mb-6 bg-red-600/20 border border-red-500 rounded-lg p-4 animate-pulse-urgent">
        <p className="text-red-500 font-semibold text-center">
          Less than a minute remains
        </p>
      </div>
    );
  }

  return (
    <div className="w-full max-w-md mx-auto mb-6 bg-orange-600/20 border border-orange-500 rounded-lg p-4 animate-pulse-warning">
      <p className="text-orange-500 font-semibold text-center">
        Your session is ending soon
      </p>
    </div>
  );
}
