import { useEffect, useState } from "react";
import { getRemainingSeconds } from "../lib/invoke";

interface CountdownState {
  remainingSeconds: number | null;
  totalSeconds: number;
  isWarning: boolean;
  isUrgent: boolean;
  isExpired: boolean;
}

export function useCountdown(
  totalMinutes: number,
  warningMinutes: number = 5,
): CountdownState {
  const [remainingSeconds, setRemainingSeconds] = useState<number | null>(null);
  const [isWarning, setIsWarning] = useState(false);
  const [isUrgent, setIsUrgent] = useState(false);
  const [isExpired, setIsExpired] = useState(false);
  const totalSeconds = totalMinutes * 60;

  useEffect(() => {
    let isActive = true;

    const fetchRemaining = async () => {
      try {
        const remaining = await getRemainingSeconds();
        if (!isActive) {
          return;
        }

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
      } catch (error) {
        if (isActive) {
          console.error("Failed to get remaining seconds:", error);
        }
      }
    };

    void fetchRemaining();
    const interval = setInterval(fetchRemaining, 1000);

    return () => {
      isActive = false;
      clearInterval(interval);
    };
  }, [warningMinutes]);

  return { remainingSeconds, totalSeconds, isWarning, isUrgent, isExpired };
}
