import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent, screen } from '@solidjs/testing-library';
import { ProfileEditForm } from '../ProfileEditForm';
import type { UserProfile, UserPreferences } from '../../../types/user';

const notify = vi.fn();

vi.mock('../../notifications/AppNotifications', () => ({
  useAppNotifications: () => ({ notify })
}));

const profile: UserProfile = {
  id: 'user-1',
  email: '  ada@example.com  ',
  displayName: 'Ada Trader',
  avatarUrl: 'https://cdn.example.com/ada.png',
  createdAt: '2026-01-01T00:00:00.000Z'
};

const preferences: UserPreferences = {
  emailNotifications: true,
  pushNotifications: true,
  inAppNotifications: true
};

function renderForm(overrides: Partial<{ submitError: string | null; isSubmitting: boolean }> = {}) {
  const onSubmit = vi.fn();
  const onAvatarSelected = vi.fn();
  const onPreferencesChange = vi.fn();
  render(() => (
    <ProfileEditForm
      profile={profile}
      preferences={preferences}
      isSubmitting={overrides.isSubmitting ?? false}
      submitError={overrides.submitError ?? null}
      onSubmit={onSubmit}
      onAvatarSelected={onAvatarSelected}
      onPreferencesChange={onPreferencesChange}
    />
  ));
  return { onSubmit, onPreferencesChange };
}

describe('ProfileEditForm', () => {
  it('pre-fills the fields from the profile and shows the avatar preview', () => {
    renderForm();
    expect((screen.getByLabelText('Display name') as HTMLInputElement).value).toBe('Ada Trader');
    expect((screen.getByLabelText('Email') as HTMLInputElement).value).toBe('  ada@example.com  ');
    const img = screen.getByAltText('Avatar preview') as HTMLImageElement;
    expect(img.src).toBe('https://cdn.example.com/ada.png');
  });

  it('blocks submission and shows validation errors for an empty profile', async () => {
    const { onSubmit } = renderForm();

    fireEvent.input(screen.getByLabelText('Display name'), { target: { value: '   ' } });
    fireEvent.input(screen.getByLabelText('Email'), { target: { value: 'oops' } });
    fireEvent.click(screen.getByRole('button'));

    expect(await screen.findByText('Display name is required')).toBeTruthy();
    expect(screen.getByText('Email is not valid')).toBeTruthy();
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it('submits trimmed typed values when valid (profile save flow)', () => {
    const { onSubmit } = renderForm();

    fireEvent.input(screen.getByLabelText('Display name'), { target: { value: '  New Name  ' } });
    fireEvent.input(screen.getByLabelText('Email'), { target: { value: ' new@example.com ' } });
    fireEvent.click(screen.getByRole('button'));

    expect(onSubmit).toHaveBeenCalledWith({ displayName: 'New Name', email: 'new@example.com' });
  });

  it('toggling a notification preference fires onPreferencesChange with the updated set', () => {
    const { onPreferencesChange } = renderForm();

    fireEvent.click(screen.getByLabelText(/In-app notifications/));
    expect(onPreferencesChange).toHaveBeenCalledWith({
      emailNotifications: true,
      pushNotifications: true,
      inAppNotifications: false
    });

    fireEvent.click(screen.getByLabelText(/Email notifications/));
    expect(onPreferencesChange).toHaveBeenCalledWith({
      emailNotifications: false,
      pushNotifications: true,
      inAppNotifications: true
    });
  });

  it('renders the server-side submit error', () => {
    renderForm({ submitError: 'Email is already registered' });
    expect(screen.getByText('Email is already registered')).toBeTruthy();
  });
});
