import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PreferencesPanel } from "./components/Preferences";
import { SourceSection } from "./components/SourceSection";
import { usePreferences } from "./hooks/usePreferences";
import { useSources } from "./hooks/useSources";
import "./App.css";

function App() {
  const { sources, error: sourcesError } = useSources();
  const { prefs, error: prefsError, save, setAutostart } = usePreferences();

  const [showPrefs, setShowPrefs] = useState(false);

  // Apply theme pref to <html>. "auto" removes the attribute so CSS falls back
  // to prefers-color-scheme; "light"/"dark" force the variables in App.css.
  useEffect(() => {
    const theme = prefs?.appearance.theme;
    const root = document.documentElement;
    if (!theme || theme === "auto") {
      root.removeAttribute("data-theme");
    } else {
      root.setAttribute("data-theme", theme);
    }
  }, [prefs?.appearance.theme]);

  const interval = prefs?.display.refresh_interval_seconds ?? 60;

  return (
    <main className="container">
      <header className="app-header">
        <h1>ClaudeView</h1>
        <button
          className="gear"
          onClick={() => setShowPrefs(true)}
          aria-label="preferences"
          title="Preferences"
        >
          ⚙
        </button>
      </header>

      <div className="sources">
        {sources.length === 0 ? (
          <div className="empty">starting…</div>
        ) : (
          sources.map((s) => <SourceSection key={s.config.sshHost ?? "__local__"} status={s} />)
        )}
      </div>

      <footer className="app-footer">
        <span className="muted">refresh every {interval}s</span>
        <button className="link-btn" onClick={() => invoke("refresh_now").catch(() => {})}>
          refresh now
        </button>
      </footer>

      {sourcesError && <p className="error">sources: {sourcesError}</p>}

      {showPrefs && prefs && (
        <PreferencesPanel
          prefs={prefs}
          onChange={save}
          onAutostartChange={setAutostart}
          onClose={() => setShowPrefs(false)}
          error={prefsError}
        />
      )}
    </main>
  );
}

export default App;
