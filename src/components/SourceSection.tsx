import type { ActiveSession, SourceSnapshot, SourceStatus } from "../types";
import { UsageBar } from "./UsageBar";

function dotClass(status: SourceStatus["state"]): string {
  if (status.status === "loading") return "dot-gray";
  if (status.status === "error") return "dot-red";
  const pct = status.data.quota?.fiveHourPct;
  if (pct == null) return "dot-gray";
  if (pct >= 95) return "dot-red";
  if (pct >= 80) return "dot-orange";
  return "dot-green";
}

function relativeTime(ts: number): string {
  const seconds = Date.now() / 1000 - ts;
  if (seconds < 60) return "just now";
  if (seconds < 3600) return `${Math.floor(seconds / 60)}min ago`;
  return `${Math.floor(seconds / 3600)}h ago`;
}

function shortProject(session: ActiveSession): string {
  return session.project ?? (session.cwd ? session.cwd.split("/").pop() ?? "?" : "?");
}

function SnapshotBody({ snapshot }: { snapshot: SourceSnapshot }) {
  const { quota, activeSessions, tokenStats } = snapshot;
  return (
    <>
      {quota ? (
        <div className="sub-bars">
          {quota.fiveHourPct != null && (
            <UsageBar label="5h" pct={quota.fiveHourPct} resetsAt={quota.fiveHourResetsAt} />
          )}
          {quota.sevenDayPct != null && (
            <UsageBar label="7d" pct={quota.sevenDayPct} resetsAt={quota.sevenDayResetsAt} />
          )}
          {quota.contextWindowPct != null && quota.modelDisplay && (
            <UsageBar label={`ctx · ${quota.modelDisplay}`} pct={quota.contextWindowPct} />
          )}
        </div>
      ) : (
        <div className="empty">no quota data — check the statusLine hook</div>
      )}

      {activeSessions.length > 0 && (
        <div className="source-sessions">
          <div className="source-sub">Active sessions</div>
          <ul className="sessions">
            {activeSessions.map((s) => (
              <li key={s.pid}>
                <span className="cwd" title={s.cwd ?? ""}>
                  {shortProject(s)}
                </span>
                <span className="pid">
                  {s.contextWindowPct != null ? `${Math.round(s.contextWindowPct)}% ctx` : ""}
                </span>
              </li>
            ))}
          </ul>
        </div>
      )}

      {tokenStats.totalMessages > 0 && (
        <p className="muted source-stats">
          last {tokenStats.windowDays}d: {tokenStats.totalMessages} msgs ·{" "}
          {tokenStats.totalToolCalls} tool calls
        </p>
      )}
    </>
  );
}

export function SourceSection({ status }: { status: SourceStatus }) {
  const { config, state } = status;
  return (
    <section className="source-card">
      <header className="source-head">
        <span className={`source-dot ${dotClass(state)}`} />
        <span className="source-title">{config.label}</span>
        {state.status === "ok" && (
          <span className="source-time">{relativeTime(state.data.generatedAt)}</span>
        )}
      </header>

      {state.status === "loading" && <div className="muted source-loading">loading…</div>}
      {state.status === "error" && <div className="error source-error">{state.data}</div>}
      {state.status === "ok" && <SnapshotBody snapshot={state.data} />}
    </section>
  );
}
