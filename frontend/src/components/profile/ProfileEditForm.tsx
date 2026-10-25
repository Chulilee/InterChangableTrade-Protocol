// Presentational profile edit form (issue #28).
//
// Pure UI + validation, mirroring the ListingForm presentation/container split:
// data loading lives in ProfileEdit, this component only renders state and
// calls back with validated inputs, which keeps it unit-testable. Avatar
// selection shows an immediate client-side preview and optimizes + uploads via
// the avatar pipeline in utils/avatar.ts.

import { createSignal, JSX, Show, For } from 'solid-js';
import { useAppNotifications } from '../notifications/AppNotifications';
import {
  hasProfileValidationErrors,
  validateProfileInput,
  type ProfileValidationErrors,
} from '../../utils/profileValidation';
import {
  ACCEPTED_AVATAR_TYPES,
  optimizeAvatarImage,
  readAvatarPreview,
  validateAvatarFile,
} from '../../utils/avatar';
import type { UserPreferences, UserProfile } from '../../types/user';

export interface ProfileEditFormProps {
  profile: UserProfile;
  preferences: UserPreferences;
  isSubmitting?: boolean;
  /** Server-side error from the last save attempt (e.g. email taken). */
  submitError?: string | null;
  /** Called with trimmed, typed profile fields when the form is valid. */
  onSubmit: (input: { displayName: string; email: string }) => void;
  /** Called with the optimized avatar image; upload is handled upstream. */
  onAvatarSelected: (file: Blob, fileName: string) => void;
  /** Called whenever a notification preference toggle changes. */
  onPreferencesChange: (next: UserPreferences) => void;
}

const PREFERENCE_ITEMS: Array<{ key: keyof UserPreferences; label: string; description: string }> = [
  {
    key: 'inAppNotifications',
    label: 'In-app notifications',
    description: 'Show success confirmations inside the app.'
  },
  {
    key: 'emailNotifications',
    label: 'Email notifications',
    description: 'Receive product and account emails.'
  },
  {
    key: 'pushNotifications',
    label: 'Push notifications',
    description: 'Receive push alerts for activity.'
  }
];

export function ProfileEditForm(props: ProfileEditFormProps) {
  const { notify } = useAppNotifications();
  const [values, setValues] = createSignal({
    displayName: props.profile.displayName,
    email: props.profile.email
  });
  const [errors, setErrors] = createSignal<ProfileValidationErrors>({});
  const [avatarPreview, setAvatarPreview] = createSignal<string | null>(
    props.profile.avatarUrl || null
  );
  const [avatarError, setAvatarError] = createSignal<string | null>(null);
  const [isProcessingAvatar, setIsProcessingAvatar] = createSignal(false);

  const handleSubmit = (event: SubmitEvent) => {
    event.preventDefault();
    const nextErrors = validateProfileInput(values());
    setErrors(nextErrors);
    if (hasProfileValidationErrors(nextErrors)) {
      return;
    }
    props.onSubmit({ displayName: values().displayName.trim(), email: values().email.trim() });
  };

  const handleAvatarChange = async (event: Event) => {
    const inputEl = event.target as HTMLInputElement;
    const file = inputEl.files?.[0];
    inputEl.value = '';
    if (!file) return;

    const validation = validateAvatarFile(file);
    if (!validation.valid) {
      setAvatarError(validation.error);
      return;
    }

    setAvatarError(null);
    setIsProcessingAvatar(true);
    try {
      // Immediate client-side preview so the user sees exactly what the
      // cropped/optimized avatar will look like.
      const preview = await readAvatarPreview(file);
      setAvatarPreview(preview);

      const optimized = await optimizeAvatarImage(file);
      props.onAvatarSelected(optimized, file.name);
      notify('Avatar ready - it persists after you save your profile');
    } catch (err) {
      setAvatarError((err as Error).message);
    } finally {
      setIsProcessingAvatar(false);
    }
  };

  const setPreference = (key: keyof UserPreferences, checked: boolean) => {
    props.onPreferencesChange({ ...props.preferences, [key]: checked });
  };

  const handleField = (field: 'displayName' | 'email') => (
    event: Event & { currentTarget: HTMLInputElement }
  ) => {
    setValues({ ...values(), [field]: event.currentTarget.value });
  };

  const labelClass = 'block text-sm font-medium text-gray-700 mb-1';
  const inputClass =
    'w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500';

  const renderField = (field: 'displayName' | 'email', label: string): JSX.Element => (
    <div>
      <label for={field} class={labelClass}>
        {label}
      </label>
      <input
        id={field}
        type={field === 'email' ? 'email' : 'text'}
        class={inputClass}
        value={values()[field]}
        onInput={handleField(field)}
      />
      <Show when={errors()[field]}>
        <p class="mt-1 text-sm text-red-600">{errors()[field]}</p>
      </Show>
    </div>
  );

  return (
    <form onSubmit={handleSubmit} class="space-y-6">
      <div class="flex items-center gap-4">
        <Show when={avatarPreview()} fallback={<div class="w-16 h-16 rounded-full bg-gray-200" />}>
          {(src) => (
            <img
              src={src()}
              alt="Avatar preview"
              class="w-16 h-16 rounded-full object-cover"
            />
          )}
        </Show>
        <div>
          <label for="avatar" class={labelClass}>
            Avatar
          </label>
          <input
            id="avatar"
            type="file"
            accept={ACCEPTED_AVATAR_TYPES.join(',')}
            class="block text-sm text-gray-700"
            onChange={handleAvatarChange}
            disabled={isProcessingAvatar()}
          />
          <p class="text-xs text-gray-500">PNG, JPEG or WebP, up to 5 MB.</p>
          <Show when={avatarError()}>
            <p class="mt-1 text-sm text-red-600">{avatarError()}</p>
          </Show>
        </div>
      </div>

      {renderField('displayName', 'Display name')}
      {renderField('email', 'Email')}

      <fieldset class="border border-gray-200 rounded-md p-4">
        <legend class="text-sm font-medium text-gray-700 px-1">Notification preferences</legend>
        <For each={PREFERENCE_ITEMS}>
          {(item) => (
            <label class="flex items-center gap-3 py-1" for={`pref-${item.key}`}>
              <input
                id={`pref-${item.key}`}
                type="checkbox"
                checked={props.preferences[item.key]}
                onChange={(event) => setPreference(item.key, event.currentTarget.checked)}
              />
              <span class="text-sm text-gray-900">{item.label}</span>
              <span class="text-xs text-gray-500">{item.description}</span>
            </label>
          )}
        </For>
      </fieldset>

      <Show when={props.submitError}>
        <p class="text-sm text-red-600">{props.submitError}</p>
      </Show>

      <button
        type="submit"
        class="px-4 py-2 bg-blue-600 text-white rounded-md font-medium disabled:opacity-50"
        disabled={props.isSubmitting}
      >
        {props.isSubmitting ? 'Saving...' : 'Save Profile'}
      </button>
    </form>
  );
}
