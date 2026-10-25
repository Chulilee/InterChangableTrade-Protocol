import { describe, it, expect } from 'vitest';
import { shouldShowToast } from '../AppNotifications';
import { DEFAULT_PREFERENCES } from '../../../types/user';

describe('shouldShowToast', () => {
  it('shows success toasts when in-app notifications are enabled', () => {
    expect(
      shouldShowToast('success', { ...DEFAULT_PREFERENCES, inAppNotifications: true })
    ).toBe(true);
  });

  it('gates success toasts on the in-app notifications preference (behavior wiring)', () => {
    expect(
      shouldShowToast('success', { ...DEFAULT_PREFERENCES, inAppNotifications: false })
    ).toBe(false);
  });

  it('always surfaces errors so problems are never silently swallowed', () => {
    expect(
      shouldShowToast('error', { ...DEFAULT_PREFERENCES, inAppNotifications: false })
    ).toBe(true);
    expect(shouldShowToast('error', DEFAULT_PREFERENCES)).toBe(true);
  });
});
