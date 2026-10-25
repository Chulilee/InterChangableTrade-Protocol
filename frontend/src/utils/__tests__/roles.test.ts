import { describe, it, expect } from 'vitest';
import { hasRequiredRole } from '../roles';

describe('hasRequiredRole', () => {
  it('grants access when no roles are required', () => {
    expect(hasRequiredRole(null, undefined)).toBe(true);
    expect(hasRequiredRole({ roles: ['user'] }, [])).toBe(true);
  });

  it('denies users without any roles claim', () => {
    expect(hasRequiredRole(null, ['admin'])).toBe(false);
    expect(hasRequiredRole(undefined, ['admin'])).toBe(false);
    expect(hasRequiredRole({ roles: undefined }, ['admin'])).toBe(false);
  });

  it('denies access when the user lacks every required role', () => {
    expect(hasRequiredRole({ roles: ['user'] }, ['admin'])).toBe(false);
    expect(hasRequiredRole({ roles: [] }, ['admin'])).toBe(false);
    expect(hasRequiredRole({ roles: ['analyst'] }, ['admin', 'moderator'])).toBe(false);
  });

  it('grants access when the user carries at least one required role', () => {
    expect(hasRequiredRole({ roles: ['admin'] }, ['admin'])).toBe(true);
    expect(hasRequiredRole({ roles: ['user', 'admin'] }, ['admin'])).toBe(true);
    expect(hasRequiredRole({ roles: ['moderator'] }, ['admin', 'moderator'])).toBe(true);
  });
});
