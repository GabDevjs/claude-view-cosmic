import { useEffect, useState } from "react";
import { PreferencesPanel } from "./components/Preferences";
import { SessionsList } from "./components/SessionsList";
import { SubscriptionBars } from "./components/SubscriptionBars";
import { TokenStatsChart } from "./components/TokenStatsChart";
import { useCache } from "./hooks/useCache";
import { usePreferences } from "./hooks/usePreferences";
import { useSessions } from "./hooks/useSessions";
import { useStats } from "./hooks/useStats";
import "./App.css";

function App() {
  const { subscription, lastUpdate, error: cacheError } = useCache();
  const { sessions, error: sessionsError } = useSessions();
  const { prefs, error: prefsError, save, setAutostart } = usePreferences();

  const historyDays = prefs?.display.history_window_days ?? 7;
  const { stats, error: statsError } = useStats(historyDays);

  const [showPrefs, setShowPrefs] = useState(false);

  // Apply theme pref to <html>. "auto" removes the attribute so CSS falls
  // back to prefers-color-scheme; "light"/"dark" force the corresponding
  // variables defined in App.css.
  useEffect(() => {
    const theme = prefs?.appearance.theme;
    const root = document.documentElement;
    if (!theme || theme === "auto") {
      root.removeAttribute("data-theme");
    } else {
      root.setAttribute("data-theme", theme);
    }
  }, [prefs?.appearance.theme]);

  const hasAnySubscription =
    subscription &&
    (subscription.fiveHourLimit !== undefined ||
      subscription.sevenDayLimit !== undefined);

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

      <p className="muted">
        {lastUpdate
          ? `last refetch: ${new Date(lastUpdate).toLocaleTimeString()}`
          : "waiting for cache…"}
      </p>

      {hasAnySubscription ? (
        <SubscriptionBars subscription={subscription} />
      ) : (
        <div className="empty">
          subscription data will appear once Claude Code writes the cache
        </div>
      )}

      <section className="sessions-section">
        <h2>Sessions</h2>
        {sessions.length === 0 ? (
          <div className="empty">no active claude sessions</div>
        ) : (
          <SessionsList sessions={sessions} />
        )}
      </section>

      <section className="sessions-section">
        <h2>Tokens · last {historyDays}d</h2>
        <TokenStatsChart stats={stats} />
      </section>

      {cacheError && <p className="error">cache: {cacheError}</p>}
      {sessionsError && <p className="error">sessions: {sessionsError}</p>}
      {statsError && <p className="error">stats: {statsError}</p>}

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
