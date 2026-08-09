import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { SourceStatus } from "../types";

/**
 * Live per-machine status. The Rust poller owns the refresh cadence and emits
 * `sources-updated`; we just re-read the shared state on each event. No timer
 * here — polling remote hosts over SSH is the backend's job so it keeps working
 * while the window is hidden.
 */
export function useSources(): { sources: SourceStatus[]; error: string | null } {
  const [sources, setSources] = useState<SourceStatus[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const refetch = () => {
      invoke<SourceStatus[]>("get_sources_status")
        .then((next) => {
          if (cancelled) return;
          setSources(next);
          setError(null);
        })
        .catch((err: unknown) => {
          if (!cancelled) setError(String(err));
        });
    };

    refetch();
    const unlisten = listen("sources-updated", refetch);
    return () => {
      cancelled = true;
      unlisten.then((fn) => fn()).catch(() => {});
    };
  }, []);

  return { sources, error };
}
