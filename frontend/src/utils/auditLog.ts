// Client-side audit trail for admin moderation actions (issue #26).
//
// The authoritative audit log lives server-side (the backend records every
// moderation action with the acting user id); this local trail complements it
// by making actions taken in this session immediately visible on the
// dashboard's activity feed even before the server's audit feed refreshes.
// Entries are appended to localStorage and capped to keep the store bounded.

export interface AdminAuditEntry {
  id: string;
  /** e.g. "user_ban", "listing_removed", "dispute_resolved". */
  action: string;
  /** Human-readable description shown in the feed. */
  message: string;
  /** Affected object id (user or listing). */
  targetId: string;
  succeeded: boolean;
  createdAt: string;
}

const AUDIT_LOG_KEY = 'admin_audit_log';
const AUDIT_LOG_LIMIT = 100;

function readRaw(): AdminAuditEntry[] {
  try {
    const raw = localStorage.getItem(AUDIT_LOG_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      if (Array.isArray(parsed)) return parsed;
    }
  } catch {
    // Corrupt store - start a fresh trail.
  }
  return [];
}

/** Returns the most recent entries, newest first. */
export function getAdminAuditLog(): AdminAuditEntry[] {
  return readRaw()
    .sort(
      (a, b) =>
        new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime()
    )
    .slice(0, AUDIT_LOG_LIMIT);
}

/** Appends one audited action and returns the entry that was recorded. */
export function recordAdminAction(entry: {
  action: string;
  message: string;
  targetId: string;
  succeeded: boolean;
  createdAt?: string;
}): AdminAuditEntry {
  const full: AdminAuditEntry = {
    id: `audit-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
    createdAt: entry.createdAt ?? new Date().toISOString(),
    ...entry
  };
  try {
    const next = [...readRaw(), full].slice(-AUDIT_LOG_LIMIT);
    localStorage.setItem(AUDIT_LOG_KEY, JSON.stringify(next));
  } catch {
    // Storage unavailable - the server still audited the action.
  }
  return full;
}

export function clearAdminAuditLog(): void {
  try {
    localStorage.removeItem(AUDIT_LOG_KEY);
  } catch {
    // Storage unavailable - nothing to clear locally.
  }
}
