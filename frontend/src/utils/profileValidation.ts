// Client-side validation for the profile edit form (issue #28).

export interface ProfileValidationErrors {
  displayName?: string;
  email?: string;
}

export interface ProfileFormValues {
  displayName: string;
  email: string;
}

export const MAX_DISPLAY_NAME_LENGTH = 64;
export const MAX_EMAIL_LENGTH = 254;
const EMAIL_PATTERN = /^[^\s@]+@[^\s@]+\.[^\s@]+$/i;

export function validateProfileInput(values: ProfileFormValues): ProfileValidationErrors {
  const errors: ProfileValidationErrors = {};

  const displayName = values.displayName.trim();
  if (!displayName) {
    errors.displayName = 'Display name is required';
  } else if (displayName.length > MAX_DISPLAY_NAME_LENGTH) {
    errors.displayName = `Display name must be at most ${MAX_DISPLAY_NAME_LENGTH} characters`;
  }

  const email = values.email.trim();
  if (!email) {
    errors.email = 'Email is required';
  } else if (email.length > MAX_EMAIL_LENGTH) {
    errors.email = `Email must be at most ${MAX_EMAIL_LENGTH} characters`;
  } else if (!EMAIL_PATTERN.test(email)) {
    errors.email = 'Email is not valid';
  }

  return errors;
}

export function hasProfileValidationErrors(errors: ProfileValidationErrors): boolean {
  return Object.keys(errors).length > 0;
}
