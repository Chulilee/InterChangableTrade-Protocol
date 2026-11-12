/**
 * Secure storage abstraction for auth/session data.
 *
 * Sensitive auth tokens must never be persisted in browser storage accessible to
 * JavaScript (e.g. localStorage/sessionStorage) to reduce XSS token theft risk.
 * This module provides a swappable in-memory implementation for access tokens.
 */

export interface InMemoryTokenRecord {
  accessToken: string;
  expiresAt: number;
}

/**
 * Swappable secure storage contract. The default implementation keeps sensitive
 * token material in memory only for the lifetime of the page/session.
 */
export interface SecureTokenStorage {
  getAccessToken(): string | null;
  getExpiresAt(): number | null;
  setTokens(tokens: InMemoryTokenRecord): void;
  clear(): void;
}

export function createMemorySecureStorage(): SecureTokenStorage {
  let record: InMemoryTokenRecord | null = null;

  return {
    getAccessToken() {
      return record?.accessToken ?? null;
    },
    getExpiresAt() {
      return record?.expiresAt ?? null;
    },
    setTokens(tokens) {
      record = {
        accessToken: tokens.accessToken,
        expiresAt: tokens.expiresAt,
      };
    },
    clear() {
      record = null;
    },
  };
}

// Default app-level secure storage (in-memory only).
export const secureTokenStorage = createMemorySecureStorage();

/**
 * Hook exposing the secure storage abstraction. Components/hooks can depend on
 * this seam instead of localStorage and swap implementations if needed.
 */
export function useSecureStorage(): SecureTokenStorage {
  return secureTokenStorage;
}

const SENSITIVE_STORAGE_KEYS = ['auth_tokens', 'accessToken', 'refreshToken'];

export function purgeSensitiveBrowserStorage(): void {
  try {
    for (const key of SENSITIVE_STORAGE_KEYS) {
      localStorage.removeItem(key);
      sessionStorage.removeItem(key);
    }
  } catch {
    // Ignore storage access errors (private mode/restricted environments).
  }
}

function readCookie(name: string): string | null {
  if (typeof document === 'undefined') return null;
  const prefix = `${name}=`;
  const parts = document.cookie ? document.cookie.split(';') : [];
  for (const part of parts) {
    const trimmed = part.trim();
    if (trimmed.startsWith(prefix)) {
      try {
        return decodeURIComponent(trimmed.slice(prefix.length));
      } catch {
        return trimmed.slice(prefix.length);
      }
    }
  }
  return null;
}

/**
 * CSRF protection helper for cookie-authenticated requests.
 * If a non-httpOnly CSRF cookie is present, attach it as a header so the
 * backend can verify same-origin intent.
 */
export function getCsrfHeaders(): Record<string, string> {
  const token =
    readCookie('XSRF-TOKEN') ||
    readCookie('csrf_token') ||
    readCookie('CSRF-TOKEN');
  return token ? { 'X-XSRF-TOKEN': token } : {};
}
