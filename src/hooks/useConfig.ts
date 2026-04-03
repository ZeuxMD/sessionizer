import { useCallback, useEffect, useState } from "react";
import { getConfig, type FrontendConfig } from "../lib/invoke";

export function useConfig() {
  const [config, setConfig] = useState<FrontendConfig | null>(null);
  const [loading, setLoading] = useState(true);

  const refetch = useCallback(async () => {
    setLoading(true);
    try {
      const cfg = await getConfig();
      setConfig(cfg);
    } catch (e) {
      console.error("Failed to get config:", e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refetch();
  }, [refetch]);

  return { config, loading, refetch };
}
