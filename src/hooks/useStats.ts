import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { TokenStats } from "../types";

const POLL_INTERVAL_MS = 60_000;

export function useStats(days: number): { stats: TokenStats | null; error: string | null } {
  const [stats, setStats] = useState<TokenStats | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const refetch = () => {
      invoke<TokenStats>("get_token_stats", { days })
        .then((next) => {
          if (cancelled) return;
          setStats(next);
          setError(null);
        })
        .catch((err: unknown) => {
          if (cancelled) return;
          setError(String(err));
        });
    };

    refetch();
    const id = window.setInterval(refetch, POLL_INTERVAL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(id);
    };
  }, [days]);

  return { stats, error };
}
