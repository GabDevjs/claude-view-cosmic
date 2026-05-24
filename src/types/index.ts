// Mirrors src-tauri/src/cache.rs. All fields are optional because the
// statusline hook only writes the PostToolUse payload — the richer schema
// (PRA §4.2) gets filled in by future phases.

export interface LimitData {
  used?: number;
  total?: number;
  resetAt?: number;
}

export interface SubscriptionData {
  fiveHourLimit?: LimitData;
  sevenDayLimit?: LimitData;
}

export interface SessionData {
  model?: string;
  contextUsed?: number;
  contextTotal?: number;
  messagesCount?: number;
}

export interface CacheData {
  updatedAt?: number;
  subscription?: SubscriptionData;
  session?: SessionData;
}

export interface Session {
  pid: number;
  cwd: string | null;
  contextUsed: number | null;
  contextTotal: number | null;
}

export interface ModelStats {
  model: string;
  messages: number;
  toolCalls: number;
  tokens: number;
}

export interface TokenStats {
  byModel: ModelStats[];
  totalMessages: number;
  totalToolCalls: number;
  totalTokens: number;
  since: number;
  days: number;
}
