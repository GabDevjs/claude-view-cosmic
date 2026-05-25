import { useState } from "react";
import { PreferencesPanel } from "./components/Preferences";
import { SessionsList } from "./components/SessionsList";
import { TokenStatsChart } from "./components/TokenStatsChart";
import { useCache } from "./hooks/useCache";
import { usePreferences } from "./hooks/usePreferences";
import { useSessions } from "./hooks/useSessions";
import { useStats } from "./hooks/useStats";
import "./App.css";

function formatLimit(used?: number, total?: number): string {
  if (used === undefined || total === undefined) return "—";
  return `${used} / ${total}`;
}

function App() {
  const { subscription, lastUpdate, error: cacheError } = useCache();
  const { sessions, error: sessionsError } = useSessions();
  const { prefs, error: prefsError, save, setAutostart } = usePreferences();

  // history_window_days propagates from prefs once they're loaded. While the
  // hook is still fetching prefs, fall back to the PRA §4.5 default so the
  // chart renders something instead of waiting.
  const historyDays = prefs?.display.history_window_days ?? 7;
  const { stats, error: statsError } = useStats(historyDays);

  const [showPrefs, setShowPrefs] = useState(false);

  return (
    <main className="container">
      <header className="app-header">
        <h1>Hello ClaudeView</h1>
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

      <dl className="stats">
        <dt>5h limit</dt>
        <dd>{formatLimit(subscription?.fiveHourLimit?.used, subscription?.fiveHourLimit?.total)}</dd>
        <dt>7d limit</dt>
        <dd>{formatLimit(subscription?.sevenDayLimit?.used, subscription?.sevenDayLimit?.total)}</dd>
      </dl>

      <section className="sessions-section">
        <h2>Sessions</h2>
        <SessionsList sessions={sessions} />
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
