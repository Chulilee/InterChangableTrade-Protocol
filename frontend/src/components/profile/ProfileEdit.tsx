// Profile edit container (issue #28): loads nothing of its own — the view and
// save flows operate on the current profile passed down from the route's
// ProfileView-derived data, the preferences store, and typed API calls. Saves
// the profile fields and avatar, then reports feedback via the in-app
// notification host (gated by the in-app notifications preference).

import { createSignal, Show } from 'solid-js';
import { useNavigate } from '@solidjs/router';
import { ProfileEditForm } from './ProfileEditForm';
import { fetchMyProfile, updateMyProfile, uploadAvatar } from '../../api/users';
import { useApi } from '../../hooks/useApi';
import { usePreferences } from '../../contexts/PreferencesContext';
import { useAppNotifications } from '../notifications/AppNotifications';
import type { UserProfile } from '../../types/user';

export function ProfileEdit(props: { initialProfile?: UserProfile }) {
  const navigate = useNavigate();
  const { apiRequest } = useApi();
  const { preferences, save } = usePreferences();
  const { notify } = useAppNotifications();

  const [profile, setProfile] = createSignal<UserProfile | null>(
    props.initialProfile ?? null
  );
  const [loadError, setLoadError] = createSignal<string | null>(null);
  const [isSubmitting, setIsSubmitting] = createSignal(false);
  const [submitError, setSubmitError] = createSignal<string | null>(null);

  // Resolve the profile for direct navigation to /profile/edit.
  void (async () => {
    if (profile()) return;
    try {
      setProfile(await fetchMyProfile(apiRequest));
    } catch (err) {
      setLoadError((err as Error).message);
    }
  })();

  const handleAvatarSelected = async (file: Blob, fileName: string) => {
    try {
      const { avatarUrl } = await uploadAvatar(file, fileName, apiRequest);
      setProfile((current) => (current ? { ...current, avatarUrl } : current));
      // The upload API persists the avatar immediately; saving the profile
      // below is only needed for display-name/email changes.
    } catch (err) {
      setSubmitError((err as Error).message);
      notify('Avatar upload failed', 'error');
    }
  };

  const handleSubmit = async (input: { displayName: string; email: string }) => {
    setIsSubmitting(true);
    setSubmitError(null);
    try {
      const saved = await updateMyProfile(input, apiRequest);
      setProfile(saved);
      notify('Profile saved');
      navigate('/profile');
    } catch (err) {
      const message = (err as Error).message;
      setSubmitError(message);
      notify(message, 'error');
    } finally {
      setIsSubmitting(false);
    }
  };

  const handlePreferencesChange = (next: Parameters<typeof save>[0]) => {
    void save(next).catch((err) => {
      const message = (err as Error).message;
      setSubmitError(message);
      notify(message, 'error');
    });
  };

  return (
    <div class="max-w-2xl mx-auto px-4 py-8">
      <h2 class="text-2xl font-bold text-gray-900 mb-6">Edit Profile</h2>
      <Show
        when={!loadError()}
        fallback={<p class="text-red-600">{loadError()}</p>}
      >
        <Show when={profile()}>
          {(current) => (
            <ProfileEditForm
              profile={current()}
              preferences={preferences()}
              isSubmitting={isSubmitting()}
              submitError={submitError()}
              onSubmit={handleSubmit}
              onAvatarSelected={handleAvatarSelected}
              onPreferencesChange={handlePreferencesChange}
            />
          )}
        </Show>
      </Show>
    </div>
  );
}
