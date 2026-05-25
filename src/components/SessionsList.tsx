import type { Session } from "../types";

function shortenCwd(cwd: string | null): string {
  if (!cwd) return "(unknown cwd)";
  const home = "/home/";
  if (cwd.startsWith(home)) {
    const rest = cwd.slice(home.length);
    const slash = rest.indexOf("/");
    return slash === -1 ? "~" : "~" + rest.slice(slash);
  }
  return cwd;
}

export function SessionsList({ sessions }: { sessions: Session[] }) {
  return (
    <ul className="sessions">
      {sessions.map((s) => (
        <li key={s.pid}>
          <span className="pid">{s.pid}</span>
          <span className="cwd" title={s.cwd ?? ""}>{shortenCwd(s.cwd)}</span>
        </li>
      ))}
    </ul>
  );
}
