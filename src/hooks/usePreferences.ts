import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Preferences } from "../types";

interface UsePreferencesState {
  prefs: Preferences | null;
  error: string | null;
  /** Persist a new full preferences object. */
  save: (next: Preferences) => Promise<void>;
  /** Toggle XDG autostart and persist the flag in one call. */
  setAutostart: (enable: boolean) => Promise<void>;
}

export function usePreferences(): UsePreferencesState {
  const [prefs, setPrefs] = useState<Preferences | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    invoke<Preferences>("get_preferences")
      .then((p) => {
        if (!cancelled) setPrefs(p);
      })
      .catch((err: unknown) => {
        if (!cancelled) setError(String(err));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const save = useCallback(async (next: Preferences) => {
    await invoke("set_preferences", { prefs: next });
    setPrefs(next);
    setError(null);
  }, []);

  const setAutostart = useCallback(
    async (enable: boolean) => {
      await invoke("toggle_autostart", { enable });
      // Backend persists the flag — refetch to stay authoritative.
      const fresh = await invoke<Preferences>("get_preferences");
      setPrefs(fresh);
      setError(null);
    },
    []
  );

  return { prefs, error, save, setAutostart };
}
