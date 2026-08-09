const ORANGE_THRESHOLD = 80;
const RED_THRESHOLD = 95;

function classFor(percent: number): string {
  if (percent > RED_THRESHOLD) return "bar-red";
  if (percent >= ORANGE_THRESHOLD) return "bar-orange";
  return "bar-green";
}

function resetIn(resetAt?: number | null): string | null {
  if (!resetAt) return null;
  const seconds = resetAt - Date.now() / 1000;
  if (seconds <= 0) return "any moment";
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${minutes}m`;
  return `${minutes}m`;
}

export function UsageBar({
  label,
  pct,
  resetsAt,
}: {
  label: string;
  pct: number;
  resetsAt?: number | null;
}) {
  const clamped = Math.min(100, Math.max(0, pct));
  const reset = resetIn(resetsAt);

  return (
    <div className="sub-bar">
      <div className="sub-bar-head">
        <span className="sub-bar-label">{label}</span>
        <span className="sub-bar-pct">{pct.toFixed(0)}%</span>
      </div>
      <div className="sub-bar-track">
        <div
          className={`sub-bar-fill ${classFor(pct)}`}
          style={{ width: `${clamped}%` }}
        />
      </div>
      {reset && (
        <div className="sub-bar-foot">
          <span />
          <span>resets in {reset}</span>
        </div>
      )}
    </div>
  );
}
