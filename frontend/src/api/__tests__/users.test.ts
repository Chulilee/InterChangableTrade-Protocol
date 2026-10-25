import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  fetchMyProfile,
  updateMyProfile,
  fetchMyPreferences,
  updateMyPreferences,
  uploadAvatar
} from '../users';
import type { UserProfile } from '../../types/user';

const mockProfile: UserProfile = {
  id: 'user-1',
  email: 'ada@example.com',
  displayName: 'Ada Trader',
  avatarUrl: null,
  createdAt: '2026-01-01T00:00:00.000Z'
};

describe('users API client', () => {
  beforeEach(() => {
    global.fetch = vi.fn();
  });

  it('fetchMyProfile returns the parsed profile on success', async () => {
    const apiRequest = vi.fn().mockResolvedValue({ ok: true, json: () => Promise.resolve(mockProfile) });
    const result = await fetchMyProfile(apiRequest);
    expect(result).toEqual(mockProfile);
    expect(apiRequest).toHaveBeenCalledWith(expect.stringContaining('/users/me'), expect.anything());
  });

  it('fetchMyProfile surfaces the server error message', async () => {
    const apiRequest = vi.fn().mockResolvedValue({ ok: false, json: () => Promise.resolve({ message: 'Session expired' }) });
    await expect(fetchMyProfile(apiRequest)).rejects.toThrow('Session expired');
  });

  it('updateMyProfile patches with the provided fields', async () => {
    const apiRequest = vi.fn().mockResolvedValue({ ok: true, json: () => Promise.resolve({ ...mockProfile, displayName: 'New Name' }) });
    const result = await updateMyProfile({ displayName: 'New Name', email: 'new@example.com' }, apiRequest);
    expect(result.displayName).toBe('New Name');
    expect(apiRequest).toHaveBeenCalledWith(
      expect.stringContaining('/users/me'),
      expect.objectContaining({
        method: 'PATCH',
        body: JSON.stringify({ displayName: 'New Name', email: 'new@example.com' })
      })
    );
  });

  it('updateMyProfile falls back to a generic message for non-JSON errors', async () => {
    const apiRequest = vi.fn().mockResolvedValue({ ok: false, json: () => Promise.reject(new Error('no json')) });
    await expect(updateMyProfile({ displayName: 'x' }, apiRequest)).rejects.toThrow('Failed to update profile');
  });

  it('fetchMyPreferences loads the saved preferences', async () => {
    const apiRequest = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ emailNotifications: false, pushNotifications: true, inAppNotifications: false })
    });
    const result = await fetchMyPreferences(apiRequest);
    expect(result.inAppNotifications).toBe(false);
    expect(apiRequest).toHaveBeenCalledWith(
      expect.stringContaining('/users/me/preferences'),
      expect.anything()
    );
  });

  it('updateMyPreferences persists preferences via PUT', async () => {
    const preferences = { emailNotifications: true, pushNotifications: false, inAppNotifications: true };
    const apiRequest = vi.fn().mockResolvedValue({ ok: true, json: () => Promise.resolve(preferences) });
    const result = await updateMyPreferences(preferences, apiRequest);
    expect(result).toEqual(preferences);
    expect(apiRequest).toHaveBeenCalledWith(
      expect.stringContaining('/users/me/preferences'),
      expect.objectContaining({ method: 'PUT', body: JSON.stringify(preferences) })
    );
  });

  it('uploadAvatar posts multipart/form-data with the avatar file', async () => {
    const apiRequest = vi.fn().mockResolvedValue({ ok: true, json: () => Promise.resolve({ avatarUrl: 'https://cdn.example.com/a.webp' }) });
    const file = new Blob(['image-bytes'], { type: 'image/webp' });
    const result = await uploadAvatar(file, 'avatar.webp', apiRequest);

    expect(result.avatarUrl).toBe('https://cdn.example.com/a.webp');
    const [url, options] = apiRequest.mock.calls[0];
    expect(url).toContain('/users/me/avatar');
    expect(options.method).toBe('POST');
    expect(options.body).toBeInstanceOf(FormData);
    const form = options.body as FormData;
    const fileEntry = form.get('avatar');
    expect(fileEntry).toBeInstanceOf(File);
    expect((fileEntry as File).name).toBe('avatar.webp');
  });

  it('uploadAvatar surfaces the server error message', async () => {
    const apiRequest = vi.fn().mockResolvedValue({ ok: false, json: () => Promise.resolve({ message: 'File too large' }) });
    const file = new Blob(['x'], { type: 'image/png' });
    await expect(uploadAvatar(file, 'a.png', apiRequest)).rejects.toThrow('File too large');
  });
});
