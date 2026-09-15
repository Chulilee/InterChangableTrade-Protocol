import { Show, createEffect } from 'solid-js';
import { useAuth } from '../../contexts/AuthContext';
import { useNavigate, useLocation } from '@solidjs/router';

interface ProtectedRouteProps {
  children: any;
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
      // Add role-based access control if needed
      // else if (props.requiredRoles && props.requiredRoles.length > 0) {
      //   const hasRequiredRole = props.requiredRoles.some(role => 
      //     state.user?.roles?.includes(role)
      //   );
      //   if (!hasRequiredRole) {
      //     navigate('/unauthorized');
      //   }
      // }
    }
  });

  return (
    <Show when={!state.isLoading && state.isAuthenticated}>
      {props.children}
    </Show>
  );
}