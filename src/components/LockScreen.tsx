import { useState } from "react";
import { CountdownTimer } from "./CountdownTimer";
import { PasswordInput } from "./PasswordInput";
import { WarningBanner } from "./WarningBanner";
import { RecoveryPrompt } from "./RecoveryPrompt";

interface LockScreenProps {
  onUnlock: () => void;
  warningMinutes: number;
}

export function LockScreen({ onUnlock, warningMinutes }: LockScreenProps) {
  const [showRecovery, setShowRecovery] = useState(false);

  const handleRecoverySuccess = () => {
    setShowRecovery(false);
    onUnlock();
  };

  return (
    <div className="min-h-screen bg-slate-900 flex items-center justify-center p-4">
      <div className="bg-slate-800/50 backdrop-blur-sm rounded-2xl p-8 shadow-2xl max-w-lg w-full text-center">
        <div className="flex items-center justify-center mb-6">
          <svg xmlns="http://www.w3.org/2000/svg" className="h-16 w-16 text-blue-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
          </svg>
        </div>

        <h1 className="text-3xl font-bold text-white mb-2">Sessionizer</h1>
        <p className="text-slate-400 mb-8">Screen time is limited</p>

        <WarningBanner isWarning={warningMinutes > 0} isUrgent={false} />

        <CountdownTimer warningMinutes={warningMinutes} />

        <div className="mt-8">
          <PasswordInput onSuccess={onUnlock} />
        </div>

        <button
          onClick={() => setShowRecovery(true)}
          className="mt-6 text-slate-500 hover:text-slate-300 text-sm"
        >
          Forgot password?
        </button>
      </div>

      {showRecovery && (
        <RecoveryPrompt
          onRecovered={handleRecoverySuccess}
          onCancel={() => setShowRecovery(false)}
        />
      )}
    </div>
  );
}
