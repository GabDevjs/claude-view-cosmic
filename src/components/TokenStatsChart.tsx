import {
  Bar,
  BarChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type { TokenStats } from "../types";

function shortModelName(full: string): string {
  // claude-opus-4-7 → opus
  // claude-sonnet-4-6 → sonnet
  // claude-haiku-4-5-20251001 → haiku
  const m = full.match(/(opus|sonnet|haiku)/i);
  return m ? m[1].toLowerCase() : full;
}

function formatTokens(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return String(n);
}

export function TokenStatsChart({ stats }: { stats: TokenStats | null }) {
  if (!stats) {
    return <div className="empty">loading stats…</div>;
  }
  if (stats.byModel.length === 0) {
    return <div className="empty">no usage in last {stats.days} day(s)</div>;
  }

  const data = stats.byModel.map((m) => ({
    model: shortModelName(m.model),
    tokens: m.tokens,
    messages: m.messages,
    toolCalls: m.toolCalls,
  }));

  return (
    <div className="stats-chart">
      <ResponsiveContainer width="100%" height={140}>
        <BarChart data={data} margin={{ top: 4, right: 8, left: 0, bottom: 4 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="#444" />
          <XAxis dataKey="model" tick={{ fill: "#888", fontSize: 11 }} />
          <YAxis tickFormatter={formatTokens} tick={{ fill: "#888", fontSize: 11 }} />
          <Tooltip
            formatter={(value: number) => formatTokens(value)}
            contentStyle={{ background: "#222", border: "1px solid #444", fontSize: "0.8em" }}
          />
          <Bar dataKey="tokens" fill="#7aa2f7" />
        </BarChart>
      </ResponsiveContainer>
      <p className="muted stats-totals">
        {stats.totalMessages} msgs · {stats.totalToolCalls} tool calls ·{" "}
        {formatTokens(stats.totalTokens)} tokens
      </p>
    </div>
  );
}
