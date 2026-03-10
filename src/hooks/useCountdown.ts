import { useState, useEffect, useCallback, useRef } from "react";
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
  const hasExecutedRef = useRef(false);

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
        const [remaining, config] = await Promise.all([
          getRemainingSeconds(),
          getConfig(),
        ]);
        setAction(config.action);
        
        if (remaining !== null) {
          setRemainingSeconds(remaining);
          setIsWarning(remaining <= warningMinutes * 60 && remaining > 60);
          setIsUrgent(remaining <= 60 && remaining > 0);
          setIsExpired(remaining <= 0);

          if (remaining <= 0 && !hasExecutedRef.current) {
            hasExecutedRef.current = true;
            try {
              await executeShutdown(config.action);
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
  }, [warningMinutes]);

  return { remainingSeconds, isWarning, isUrgent, isExpired };
}
