// Types for the User Profile & Settings feature (issue #28).
//
// These mirror the shape returned by the backend REST API, following the same
// convention as types/listing.ts: the frontend talks to a remote API via
// `VITE_API_URL` (see AuthContext.tsx), so these types describe the contract
// the frontend expects from it.

export interface UserProfile {
  id: string;
  email: string;
  displayName: string;
  /** Publicly viewable avatar URL, or null when none has been uploaded. */
  avatarUrl?: string | null;
  /** ISO timestamp of when the account was created (shown as "member since"). */
  createdAt: string;
}

/** Fields a user may edit on their own profile. */
export interface UpdateProfileInput {
  displayName?: string;
  email?: string;
}

/** User-configurable notification preferences. */
export interface UserPreferences {
  /** Receive transactional/product emails. */
  emailNotifications: boolean;
  /** Receive push notifications. */
  pushNotifications: boolean;
  /** Show in-app notification toasts. */
  inAppNotifications: boolean;
}

export const DEFAULT_PREFERENCES: UserPreferences = {
  emailNotifications: true,
  pushNotifications: true,
  inAppNotifications: true,
};

/** Response of a successful avatar upload. */
export interface AvatarUploadResult {
  /** Persistent URL of the stored avatar, as returned by the upload API. */
  avatarUrl: string;
}
