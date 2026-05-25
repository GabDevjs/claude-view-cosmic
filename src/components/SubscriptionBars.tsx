import type { LimitData, SubscriptionData } from "../types";

const ORANGE_THRESHOLD = 80;
const RED_THRESHOLD = 95;

function classFor(percent: number): string {
  if (percent > RED_THRESHOLD) return "bar-red";
  if (percent >= ORANGE_THRESHOLD) return "bar-orange";
  return "bar-green";
}

function pct(limit?: LimitData): number | null {
  if (!limit || limit.total === undefined || limit.total === 0) return null;
  return ((limit.used ?? 0) / limit.total) * 100;
}

function resetIn(resetAt?: number): string | null {
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

function Bar({ label, limit }: { label: string; limit?: LimitData }) {
  const percent = pct(limit);
  const reset = resetIn(limit?.resetAt);

  return (
    <div className="sub-bar">
      <div className="sub-bar-head">
        <span className="sub-bar-label">{label}</span>
        <span className="sub-bar-pct">
          {percent === null ? "—" : `${percent.toFixed(0)}%`}
        </span>
      </div>
      <div className="sub-bar-track">
        {percent !== null && (
          <div
            className={`sub-bar-fill ${classFor(percent)}`}
            style={{ width: `${Math.min(100, percent)}%` }}
          />
        )}
      </div>
      <div className="sub-bar-foot">
        <span>
          {limit?.used !== undefined && limit?.total !== undefined
            ? `${limit.used} / ${limit.total}`
            : "no data yet"}
        </span>
        {reset && <span>resets in {reset}</span>}
      </div>
    </div>
  );
}

export function SubscriptionBars({ subscription }: { subscription: SubscriptionData | null }) {
  return (
    <div className="sub-bars">
      <Bar label="5h" limit={subscription?.fiveHourLimit} />
      <Bar label="7d" limit={subscription?.sevenDayLimit} />
    </div>
  );
}
