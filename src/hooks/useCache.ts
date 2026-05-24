import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { SubscriptionData } from "../types";

interface UseCacheState {
  subscription: SubscriptionData | null;
  lastUpdate: number | null;
  error: string | null;
}

export function useCache(): UseCacheState {
  const [state, setState] = useState<UseCacheState>({
    subscription: null,
    lastUpdate: null,
    error: null,
  });

  const refetch = useCallback(() => {
    invoke<SubscriptionData | null>("get_subscription_data")
      .then((subscription) => {
        setState({ subscription, lastUpdate: Date.now(), error: null });
      })
      .catch((err: unknown) => {
        setState((prev) => ({ ...prev, error: String(err) }));
      });
  }, []);

  useEffect(() => {
    refetch();
    const unlistenPromise = listen("cache-updated", () => refetch());
    return () => {
      unlistenPromise.then((fn) => fn()).catch(() => {});
    };
  }, [refetch]);

  return state;
}
