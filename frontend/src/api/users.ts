// Typed user profile API for the Profile & Settings feature (issue #28).
//
// Follows the same conventions as api/listings.ts: a `VITE_API_URL` env var
// (defaulting to the local dev proxy target) and an `apiRequest` function
// produced by `useApi()` for authenticated calls, so token attachment/refresh
// stays centralized in one place.

import type {
  AvatarUploadResult,
  UpdateProfileInput,
  UserPreferences,
  UserProfile,
} from '../types/user';

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
    // Response body wasn't JSON (or was empty) - stick with the fallback.
  }
  throw new Error(message);
}

/** Fetches the authenticated user's profile. */
export async function fetchMyProfile(apiRequest: AuthedRequest): Promise<UserProfile> {
  const response = await apiRequest(`${API_BASE_URL}/users/me`);
  if (!response.ok) {
    await throwForErrorResponse(response, 'Failed to load profile');
  }
  return response.json();
}

/** Updates the authenticated user's profile fields. */
export async function updateMyProfile(
  input: UpdateProfileInput,
  apiRequest: AuthedRequest
): Promise<UserProfile> {
  const response = await apiRequest(`${API_BASE_URL}/users/me`, {
    method: 'PATCH',
    body: JSON.stringify(input)
  });
  if (!response.ok) {
    await throwForErrorResponse(response, 'Failed to update profile');
  }
  return response.json();
}

/** Fetches the authenticated user's saved notification preferences. */
export async function fetchMyPreferences(apiRequest: AuthedRequest): Promise<UserPreferences> {
  const response = await apiRequest(`${API_BASE_URL}/users/me/preferences`);
  if (!response.ok) {
    await throwForErrorResponse(response, 'Failed to load preferences');
  }
  return response.json();
}

/** Persists the authenticated user's notification preferences. */
export async function updateMyPreferences(
  preferences: UserPreferences,
  apiRequest: AuthedRequest
): Promise<UserPreferences> {
  const response = await apiRequest(`${API_BASE_URL}/users/me/preferences`, {
    method: 'PUT',
    body: JSON.stringify(preferences)
  });
  if (!response.ok) {
    await throwForErrorResponse(response, 'Failed to save preferences');
  }
  return response.json();
}

/**
 * Uploads an optimized avatar image as multipart/form-data and returns the
 * stored avatar URL. The caller is responsible for preparing an optimized
 * image (see utils/avatar.ts); the authed request must not force a JSON
 * Content-Type for FormData bodies (see useApi).
 */
export async function uploadAvatar(
  file: Blob,
  fileName: string,
  apiRequest: AuthedRequest
): Promise<AvatarUploadResult> {
  const form = new FormData();
  form.append('avatar', file, fileName);

  const response = await apiRequest(`${API_BASE_URL}/users/me/avatar`, {
    method: 'POST',
    body: form
  });
  if (!response.ok) {
    await throwForErrorResponse(response, 'Failed to upload avatar');
  }
  return response.json();
}
