import { useState, useEffect } from "react";
import { getRemainingSeconds, getConfig } from "../lib/invoke";

interface CountdownState {
  remainingSeconds: number | null;
  totalSeconds: number;
  isWarning: boolean;
  isUrgent: boolean;
  isExpired: boolean;
}

export function useCountdown(warningMinutes: number = 5): CountdownState {
  const [remainingSeconds, setRemainingSeconds] = useState<number | null>(null);
  const [totalSeconds, setTotalSeconds] = useState(0);
  const [isWarning, setIsWarning] = useState(false);
  const [isUrgent, setIsUrgent] = useState(false);
  const [isExpired, setIsExpired] = useState(false);

  useEffect(() => {
    let isActive = true;
    const fetchRemaining = async () => {
      try {
        const [remaining, config] = await Promise.all([
          getRemainingSeconds(),
          getConfig(),
        ]);
        if (!isActive) return;
        setTotalSeconds(config.timeout_minutes * 60);

        if (remaining !== null) {
          setRemainingSeconds(remaining);
          setIsWarning(remaining <= warningMinutes * 60 && remaining > 60);
          setIsUrgent(remaining <= 60 && remaining > 0);
          setIsExpired(remaining <= 0);
        } else {
          setRemainingSeconds(null);
          setIsWarning(false);
          setIsUrgent(false);
          setIsExpired(false);
        }
      } catch (e) {
        if (isActive) {
          console.error("Failed to get remaining seconds:", e);
        }
      }
    };

    fetchRemaining();
    const interval = setInterval(fetchRemaining, 1000);
    return () => {
      isActive = false;
      clearInterval(interval);
    };
  }, [warningMinutes]);

  return { remainingSeconds, totalSeconds, isWarning, isUrgent, isExpired };
}
