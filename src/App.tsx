import { SessionsList } from "./components/SessionsList";
import { TokenStatsChart } from "./components/TokenStatsChart";
import { useCache } from "./hooks/useCache";
import { useSessions } from "./hooks/useSessions";
import { useStats } from "./hooks/useStats";
import "./App.css";

const HISTORY_DAYS = 7;

function formatLimit(used?: number, total?: number): string {
  if (used === undefined || total === undefined) return "—";
  return `${used} / ${total}`;
}

function App() {
  const { subscription, lastUpdate, error: cacheError } = useCache();
  const { sessions, error: sessionsError } = useSessions();
  const { stats, error: statsError } = useStats(HISTORY_DAYS);

  return (
    <main className="container">
      <h1>Hello ClaudeView</h1>

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
        <h2>Tokens · last {HISTORY_DAYS}d</h2>
        <TokenStatsChart stats={stats} />
      </section>

      {cacheError && <p className="error">cache: {cacheError}</p>}
      {sessionsError && <p className="error">sessions: {sessionsError}</p>}
      {statsError && <p className="error">stats: {statsError}</p>}
    </main>
  );
}

export default App;
