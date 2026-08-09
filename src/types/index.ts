// Mirrors the JSON emitted by scripts/collect_status.py (decoded in Rust by
// src-tauri/src/collector.rs). One SourceSnapshot per monitored machine.

export interface SourceConfig {
  label: string;
  /** null for the local machine; a ~/.ssh/config alias for a remote. */
  sshHost: string | null;
}

export interface Quota {
  model?: string | null;
  modelDisplay?: string | null;
  fiveHourPct?: number | null;
  fiveHourResetsAt?: number | null;
  sevenDayPct?: number | null;
  sevenDayResetsAt?: number | null;
  contextWindowPct?: number | null;
  updatedAt?: number | null;
}

export interface ActiveSession {
  pid: number;
  cwd?: string | null;
  project?: string | null;
  runningFor?: string | null;
  sessionId?: string | null;
  lastModified?: number | null;
  contextWindowPct?: number | null;
}

export interface ModelStats {
  messages: number;
  toolCalls: number;
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheCreationTokens: number;
}

export interface TokenStats {
  windowDays: number;
  totalMessages: number;
  totalToolCalls: number;
  byModel: Record<string, ModelStats>;
}

export interface SourceSnapshot {
  hostname: string;
  generatedAt: number;
  quota?: Quota | null;
  activeSessions: ActiveSession[];
  tokenStats: TokenStats;
}

/** Discriminated union mirroring the Rust `SourceState` enum. */
export type SourceState =
  | { status: "loading" }
  | { status: "ok"; data: SourceSnapshot }
  | { status: "error"; data: string };

export interface SourceStatus {
  config: SourceConfig;
  state: SourceState;
}

// Preferences — mirrors src-tauri/src/config.rs (snake_case fields).

export interface DisplayPrefs {
  show_percentage: boolean;
  alert_threshold: number;
  history_window_days: number;
  refresh_interval_seconds: number;
}

export interface BehaviorPrefs {
  autostart: boolean;
  notifications_enabled: boolean;
  notification_thresholds: number[];
}

export interface AppearancePrefs {
  theme: string;
}

export interface Preferences {
  remote_hosts: string[];
  display: DisplayPrefs;
  behavior: BehaviorPrefs;
  appearance: AppearancePrefs;
}
