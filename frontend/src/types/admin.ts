// Types for the Admin dashboard & moderation tools feature (issue #26).
//
// These mirror the shape returned by the backend admin REST API (the service
// lives in InterChangableTrade-Core); the frontend talks to it via
// `VITE_API_URL`, so these types describe the contract the frontend expects
// from it — same convention as types/listing.ts and types/user.ts.

export interface AdminUserSummary {
  id: string;
  email: string;
  displayName?: string;
  avatarUrl?: string | null;
  /** Roles assigned to the account (e.g. ["admin"]). */
  roles: string[];
  /** Whether the account is banned from signing in and listing. */
  banned: boolean;
  createdAt: string;
}

export interface AdminDashboardStats {
  totalUsers: number;
  activeUsers: number;
  /** Listings awaiting moderation. */
  pendingListings: number;
  /** Disputes not yet resolved. */
  openDisputes: number;
}

/** A listing awaiting moderation. `status` is pending until actioned. */
export interface ModerationItem {
  id: string;
  title: string;
  sellerId: string;
  sellerName?: string;
  /** ISO timestamp the listing was submitted for review. */
  submittedAt: string;
  status: 'pending' | 'approved' | 'removed';
}

export interface AdminUsersQuery {
  page?: number;
  limit?: number;
  search?: string;
  banned?: boolean;
}

export interface Paginated<Items> {
  items: Items[];
  total: number;
  page: number;
  limit: number;
}

/** One entry of the admin activity feed. */
export interface ActivityEntry {
  id: string;
  /** Discriminator of what happened (listing_removed, user_ban, ...). */
  type: string;
  message: string;
  /** Who performed the action, when applicable. */
  actorId?: string;
  createdAt: string;
}

/** Payload accepted by the dispute resolution endpoint. */
export interface ResolveDisputeInput {
  outcome: 'seller' | 'buyer' | 'split' | 'dismissed';
  resolutionNote: string;
}
