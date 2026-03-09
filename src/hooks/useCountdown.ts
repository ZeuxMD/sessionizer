import { useState, useEffect, useCallback } from "react";
import {
  getRemainingSeconds,
  executeShutdown,
  getConfig,
} from "../lib/invoke";

interface CountdownState {
  remainingSeconds: number | null;
  isWarning: boolean;
  isUrgent: boolean;
  isExpired: boolean;
}

export function useCountdown(warningMinutes: number = 5): CountdownState {
  const [remainingSeconds, setRemainingSeconds] = useState<number | null>(null);
  const [isWarning, setIsWarning] = useState(false);
  const [isUrgent, setIsUrgent] = useState(false);
  const [isExpired, setIsExpired] = useState(false);
  const [action, setAction] = useState<string>("shutdown");

  const checkAndExecute = useCallback(async () => {
    try {
      const config = await getConfig();
      setAction(config.action);
    } catch (e) {
      console.error("Failed to get config:", e);
    }
  }, []);

  useEffect(() => {
    checkAndExecute();
  }, [checkAndExecute]);

  useEffect(() => {
    const fetchRemaining = async () => {
      try {
        const remaining = await getRemainingSeconds();
        if (remaining !== null) {
          setRemainingSeconds(remaining);
          setIsWarning(remaining <= warningMinutes * 60 && remaining > 60);
          setIsUrgent(remaining <= 60 && remaining > 0);
          setIsExpired(remaining <= 0);

          if (remaining <= 0) {
            try {
              await executeShutdown(action);
            } catch (e) {
              console.error("Failed to execute shutdown:", e);
            }
          }
        }
      } catch (e) {
        console.error("Failed to get remaining seconds:", e);
      }
    };

    fetchRemaining();
    const interval = setInterval(fetchRemaining, 1000);
    return () => clearInterval(interval);
  }, [warningMinutes, action]);

  return { remainingSeconds, isWarning, isUrgent, isExpired };
}
