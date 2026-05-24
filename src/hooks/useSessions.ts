import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Session } from "../types";

const POLL_INTERVAL_MS = 5_000;

export function useSessions(): { sessions: Session[]; error: string | null } {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const refetch = () => {
      invoke<Session[]>("get_active_sessions")
        .then((next) => {
          if (cancelled) return;
          setSessions(next);
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
  }, []);

  return { sessions, error };
}
