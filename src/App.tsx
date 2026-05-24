import { SessionsList } from "./components/SessionsList";
import { useCache } from "./hooks/useCache";
import { useSessions } from "./hooks/useSessions";
import "./App.css";

function formatLimit(used?: number, total?: number): string {
  if (used === undefined || total === undefined) return "—";
  return `${used} / ${total}`;
}

function App() {
  const { subscription, lastUpdate, error: cacheError } = useCache();
  const { sessions, error: sessionsError } = useSessions();

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

      {cacheError && <p className="error">cache: {cacheError}</p>}
      {sessionsError && <p className="error">sessions: {sessionsError}</p>}
    </main>
  );
}

export default App;
