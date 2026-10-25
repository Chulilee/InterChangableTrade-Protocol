import { describe, it, expect } from 'vitest';
import {
  validateProfileInput,
  hasProfileValidationErrors,
  MAX_DISPLAY_NAME_LENGTH
} from '../profileValidation';

describe('validateProfileInput', () => {
  it('accepts a valid profile', () => {
    const errors = validateProfileInput({
      displayName: 'Ada Trader',
      email: 'ada@example.com'
    });
    expect(hasProfileValidationErrors(errors)).toBe(false);
  });

  it('trims fields before validating', () => {
    const errors = validateProfileInput({
      displayName: '  Ada  ',
      email: '  ada@example.com  '
    });
    expect(hasProfileValidationErrors(errors)).toBe(false);
  });

  it('requires a display name', () => {
    const errors = validateProfileInput({ displayName: '   ', email: 'a@b.co' });
    expect(errors.displayName).toBe('Display name is required');
  });

  it('caps the display name length', () => {
    const errors = validateProfileInput({
      displayName: 'x'.repeat(MAX_DISPLAY_NAME_LENGTH + 1),
      email: 'a@b.co'
    });
    expect(errors.displayName).toBe('Display name must be at most 64 characters');
  });

  it('requires an email', () => {
    const errors = validateProfileInput({ displayName: 'Ada', email: '' });
    expect(errors.email).toBe('Email is required');
  });

  it('rejects malformed emails', () => {
    expect(validateProfileInput({ displayName: 'Ada', email: 'not-an-email' }).email).toBe(
      'Email is not valid'
    );
    expect(validateProfileInput({ displayName: 'Ada', email: 'a@b' }).email).toBe(
      'Email is not valid'
    );
  });

  it('caps the email length', () => {
    const errors = validateProfileInput({
      displayName: 'Ada',
      email: `${'x'.repeat(250)}@example.com`
    });
    expect(errors.email).toContain('254');
  });
});
