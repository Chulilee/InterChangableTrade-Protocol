// Profile page container (issue #28): loads the authenticated user's profile
// and renders the read-only view, linking to the edit page.

import { createResource, Show } from 'solid-js';
import { A } from '@solidjs/router';
import { fetchMyProfile } from '../../api/users';
import { useApi } from '../../hooks/useApi';
import type { UserProfile } from '../../types/user';

function fmtMemberSince(profile: UserProfile): string {
  try {
    return new Date(profile.createdAt).toLocaleDateString();
  } catch {
    return profile.createdAt;
  }
}

export function ProfileView() {
  const { apiRequest } = useApi();
  const [profile, { refetch }] = createResource(async () => fetchMyProfile(apiRequest));

  return (
    <div class="max-w-2xl mx-auto px-4 py-8">
      <h2 class="text-2xl font-bold text-gray-900 mb-6">Your Profile</h2>
      <Show
        when={!profile.loading}
        fallback={<p class="text-gray-500">Loading profile...</p>}
      >
        <Show
          when={profile()}
          fallback={<p class="text-red-600">Failed to load your profile.</p>}
        >
          {(current) => (
            <div class="space-y-4">
              <div class="flex items-center gap-4">
                <Show
                  when={current().avatarUrl}
                  fallback={<div class="w-16 h-16 rounded-full bg-gray-200" />}
                >
                  <img
                    src={current().avatarUrl as string}
                    alt="Your avatar"
                    class="w-16 h-16 rounded-full object-cover"
                  />
                </Show>
                <div>
                  <p class="text-lg font-semibold text-gray-900">{current().displayName}</p>
                  <p class="text-sm text-gray-600">{current().email}</p>
                  <p class="text-xs text-gray-500">
                    Member since {fmtMemberSince(current())}
                  </p>
                </div>
              </div>
              <A
                href="/profile/edit"
                class="inline-block px-4 py-2 bg-blue-600 text-white rounded-md font-medium"
              >
                Edit Profile
              </A>
              <button
                type="button"
                class="ml-2 px-4 py-2 border border-gray-300 rounded-md text-gray-700"
                onClick={() => void refetch()}
              >
                Refresh
              </button>
            </div>
          )}
        </Show>
      </Show>
    </div>
  );
}
