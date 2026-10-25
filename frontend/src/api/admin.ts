// Admin dashboard & moderation API (issue #26).
//
// Follows the same conventions as api/listings.ts: a `VITE_API_URL` env var
// and an `apiRequest` produced by `useApi()`; every call here is
// admin-privileged, and the backend enforces the role on each route (the
// frontend guard is defense-in-depth, see ProtectedRoute + utils/roles.ts).

import type {
  ActivityEntry,
  AdminDashboardStats,
  AdminUsersQuery,
  ModerationItem,
  Paginated,
  ResolveDisputeInput
} from '../types/admin';

const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:3000/api';

/** Matches the shape of `useApi().apiRequest`. */
export type AuthedRequest = (url: string, options?: RequestInit) => Promise<Response>;

async function throwForErrorResponse(response: Response, fallback: string): Promise<never> {
  let message = fallback;
  try {
    const data = await response.json();
    if (data && typeof data.message === 'string') {
      message = data.message;
    }
  } catch {
    // Non-JSON error body - stick with the fallback.
  }
  throw new Error(message);
}

export function buildAdminUsersQuery(query: AdminUsersQuery = {}): string {
  const params = new URLSearchParams();
  if (query.page) params.set('page', String(query.page));
  if (query.limit) params.set('limit', String(query.limit));
  if (query.search) params.set('search', query.search);
  if (query.banned !== undefined) params.set('banned', String(query.banned));
  const qs = params.toString();
  return qs ? `?${qs}` : '';
}

/** Aggregate counts for the dashboard header cards. */
export async function fetchAdminStats(apiRequest: AuthedRequest): Promise<AdminDashboardStats> {
  const response = await apiRequest(`${API_BASE_URL}/admin/stats`);
  if (!response.ok) await throwForErrorResponse(response, 'Failed to load admin stats');
  return response.json();
}

/** Paginated user list with moderation-relevant fields. */
export async function fetchAdminUsers(
  query: AdminUsersQuery,
  apiRequest: AuthedRequest
): Promise<Paginated<AdminUserSummary>> {
  const response = await apiRequest(`${API_BASE_URL}/admin/users${buildAdminUsersQuery(query)}`);
  if (!response.ok) await throwForErrorResponse(response, 'Failed to load users');
  return response.json();
}

/** Listings awaiting moderation. */
export async function fetchModerationQueue(
  apiRequest: AuthedRequest
): Promise<ModerationItem[]> {
  const response = await apiRequest(`${API_BASE_URL}/admin/moderation-queue`);
  if (!response.ok) await throwForErrorResponse(response, 'Failed to load the moderation queue');
  const data = await response.json();
  return data.items ?? data;
}

/** Recent admin/current activity for the feed. */
export async function fetchActivityFeed(apiRequest: AuthedRequest): Promise<ActivityEntry[]> {
  const response = await apiRequest(`${API_BASE_URL}/admin/activity`);
  if (!response.ok) await throwForErrorResponse(response, 'Failed to load the activity feed');
  const data = await response.json();
  return data.items ?? data;
}

/** Bans a user from signing in and listing. Audited server-side. */
export async function banUser(targetId: string, apiRequest: AuthedRequest): Promise<void> {
  const response = await apiRequest(`${API_BASE_URL}/admin/users/${encodeURIComponent(targetId)}/ban`, {
    method: 'POST'
  });
  if (!response.ok) await throwForErrorResponse(response, 'Failed to ban user');
}

/** Lifts a ban. Audited server-side. */
export async function unbanUser(targetId: string, apiRequest: AuthedRequest): Promise<void> {
  const response = await apiRequest(
    `${API_BASE_URL}/admin/users/${encodeURIComponent(targetId)}/unban`,
    { method: 'POST' }
  );
  if (!response.ok) await throwForErrorResponse(response, 'Failed to unban user');
}

/** Removes (unpublishes) a non-compliant listing. Audited server-side. */
export async function removeListing(targetId: string, apiRequest: AuthedRequest): Promise<void> {
  const response = await apiRequest(
    `${API_BASE_URL}/admin/listings/${encodeURIComponent(targetId)}`,
    { method: 'DELETE' }
  );
  if (!response.ok) await throwForErrorResponse(response, 'Failed to remove listing');
}

/** Approves a pending listing out of the moderation queue. Audited server-side. */
export async function approveListing(targetId: string, apiRequest: AuthedRequest): Promise<void> {
  const response = await apiRequest(
    `${API_BASE_URL}/admin/listings/${encodeURIComponent(targetId)}/approve`,
    { method: 'POST' }
  );
  if (!response.ok) await throwForErrorResponse(response, 'Failed to approve listing');
}

/** Resolves a dispute with an explicit outcome and a required note. */
export async function resolveDispute(
  disputeId: string,
  input: ResolveDisputeInput,
  apiRequest: AuthedRequest
): Promise<void> {
  const response = await apiRequest(
    `${API_BASE_URL}/admin/disputes/${encodeURIComponent(disputeId)}/resolve`,
    { method: 'POST', body: JSON.stringify(input) }
  );
  if (!response.ok) await throwForErrorResponse(response, 'Failed to resolve dispute');
}
