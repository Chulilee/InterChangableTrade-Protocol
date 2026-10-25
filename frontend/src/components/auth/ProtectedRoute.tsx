import { Show, createEffect } from 'solid-js';
import { useAuth } from '../../contexts/AuthContext';
import { useNavigate, useLocation } from '@solidjs/router';
import { hasRequiredRole } from '../../utils/roles';

interface ProtectedRouteProps {
  children: any;
  /**
   * Roles required to view this route (issue #26). Omit for
   * "any authenticated user"; e.g. ['admin'] restricts to admins. Non-owners
   * see a clear access-denied message instead of the guarded content.
   */
  requiredRoles?: string[];
}

export function ProtectedRoute(props: ProtectedRouteProps) {
  const { state } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();

  createEffect(() => {
    // Wait until loading is complete
    if (!state.isLoading) {
      if (!state.isAuthenticated) {
        // Store the current location to redirect back after login
        sessionStorage.setItem('redirectAfterLogin', location.pathname);
        navigate('/sign-in');
      }
      // Role-based access control (issue #26): the rendered output keeps a
      // non-privileged user out of guarded content via the <Show> fallback
      // below; no navigation needed so the URL stays shareable for staff.
    }
  });

  return (
    <Show when={!state.isLoading && state.isAuthenticated}>
      <Show
        when={hasRequiredRole(state.user, props.requiredRoles)}
        fallback={
          <div class="max-w-2xl mx-auto px-4 py-16 text-center">
            <h2 class="text-2xl font-bold text-gray-900 mb-2">Access denied</h2>
            <p class="text-gray-600">
              You do not have the required role to view this page
              {props.requiredRoles?.length ? ` (requires: ${props.requiredRoles.join(', ')})` : ''}.
            </p>
          </div>
        }
      >
        {props.children}
      </Show>
    </Show>
  );
}
