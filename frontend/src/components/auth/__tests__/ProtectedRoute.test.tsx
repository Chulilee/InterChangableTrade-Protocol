import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@solidjs/testing-library';
import { ProtectedRoute } from '../ProtectedRoute';

const authState = {
  isLoading: false,
  isAuthenticated: true,
  user: null as null | { id: string; name: string; email: string; roles?: string[]; createdAt: string }
};

vi.mock('../../contexts/AuthContext', () => ({
  useAuth: () => ({ state: authState })
}));

vi.mock('@solidjs/router', () => ({
  useNavigate: () => () => {},
  useLocation: () => ({ pathname: '/admin' })
}));

function currentUser(roles?: string[]) {
  authState.user = {
    id: 'u1',
    name: 'Ada',
    email: 'ada@example.com',
    roles,
    createdAt: '2026-01-01T00:00:00.000Z'
  };
}

describe('ProtectedRoute (role-based access control, issue #26)', () => {
  it('renders guarded content for a user with the required role', () => {
    currentUser(['admin']);
    render(() => (
      <ProtectedRoute requiredRoles={['admin']}>
        <p>Admin only content</p>
      </ProtectedRoute>
    ));
    expect(screen.getByText('Admin only content')).toBeTruthy();
  });

  it('denies access for a non-admin user, hiding the guarded content', () => {
    currentUser(['user']);
    render(() => (
      <ProtectedRoute requiredRoles={['admin']}>
        <p>Admin only content</p>
      </ProtectedRoute>
    ));
    expect(screen.queryByText('Admin only content')).toBeNull();
    expect(screen.getByText('Access denied')).toBeTruthy();
    expect(screen.getByText(/requires: admin/)).toBeTruthy();
  });

  it('denies access when the user carries no roles claim at all', () => {
    currentUser(undefined);
    render(() => (
      <ProtectedRoute requiredRoles={['admin']}>
        <p>Admin only content</p>
      </ProtectedRoute>
    ));
    expect(screen.queryByText('Admin only content')).toBeNull();
  });

  it('allows any authenticated user when no roles are required', () => {
    currentUser(undefined);
    render(() => (
      <ProtectedRoute>
        <p>Any user content</p>
      </ProtectedRoute>
    ));
    expect(screen.getByText('Any user content')).toBeTruthy();
  });
});
