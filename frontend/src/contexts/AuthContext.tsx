import { createContext, useContext, createSignal, createEffect, Accessor } from 'solid-js';
import { createStore, Store, SetStoreFunction } from 'solid-js/store';

// Types
export interface User {
  id: string;
  email: string;
  name: string;
  createdAt: string;
}

export interface AuthTokens {
  accessToken: string;
  refreshToken: string;
  expiresAt: number;
}

interface AuthState {
  user: User | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  error: string | null;
}

interface AuthContextType {
  state: Store<AuthState>;
  setState: SetStoreFunction<AuthState>;
  currentUser: Accessor<User | null>;
  signIn: (email: string, password: string) => Promise<void>;
  signUp: (email: string, password: string, name: string) => Promise<void>;
  signOut: () => Promise<void>;
  refreshToken: () => Promise<boolean>;
  forgotPassword: (email: string) => Promise<void>;
  resetPassword: (token: string, newPassword: string) => Promise<void>;
}

// API endpoints - configure these based on your backend
const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:3000/api';
const TOKEN_REFRESH_THRESHOLD = 5 * 60 * 1000; // Refresh 5 minutes before expiration

const AuthContext = createContext<AuthContextType>();

export function AuthProvider(props: { children: any }) {
  const [state, setState] = createStore<AuthState>({
    user: null,
    isAuthenticated: false,
    isLoading: true,
    error: null
  });

  const [currentUser, setCurrentUser] = createSignal<User | null>(null);

  // Secure token storage functions
  const getStoredTokens = (): AuthTokens | null => {
    try {
      const tokens = localStorage.getItem('auth_tokens');
      if (tokens) {
        return JSON.parse(tokens);
      }
      return null;
    } catch {
      return null;
    }
  };

  const storeTokens = (tokens: AuthTokens) => {
    try {
      localStorage.setItem('auth_tokens', JSON.stringify(tokens));
    } catch (error) {
      console.error('Failed to store tokens:', error);
    }
  };

  const clearTokens = () => {
    try {
      localStorage.removeItem('auth_tokens');
    } catch (error) {
      console.error('Failed to clear tokens:', error);
    }
  };

  // Token refresh logic
  const refreshToken = async (): Promise<boolean> => {
    const tokens = getStoredTokens();
    if (!tokens?.refreshToken) {
      return false;
    }

    try {
      const response = await fetch(`${API_BASE_URL}/auth/refresh`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ refreshToken: tokens.refreshToken }),
        credentials: 'include'
      });

      if (!response.ok) {
        throw new Error('Token refresh failed');
      }

      const data = await response.json();
      const newTokens: AuthTokens = {
        accessToken: data.accessToken,
        refreshToken: data.refreshToken || tokens.refreshToken,
        expiresAt: Date.now() + data.expiresIn * 1000
      };
      storeTokens(newTokens);
      return true;
    } catch (error) {
      console.error('Token refresh error:', error);
      clearTokens();
      setState({ isAuthenticated: false, user: null, isLoading: false });
      setCurrentUser(null);
      return false;
    }
  };

  // Get valid access token (refreshes if needed)
  const getValidAccessToken = async (): Promise<string | null> => {
    const tokens = getStoredTokens();
    if (!tokens) return null;

    // Check if token needs refreshing
    if (Date.now() >= tokens.expiresAt - TOKEN_REFRESH_THRESHOLD) {
      const refreshed = await refreshToken();
      if (!refreshed) return null;
      const newTokens = getStoredTokens();
      return newTokens?.accessToken || null;
    }

    return tokens.accessToken;
  };

  // Sign in function
  const signIn = async (email: string, password: string) => {
    setState({ isLoading: true, error: null });
    try {
      const response = await fetch(`${API_BASE_URL}/auth/login`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ email, password }),
        credentials: 'include'
      });

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Login failed');
      }

      const data = await response.json();
      const tokens: AuthTokens = {
        accessToken: data.accessToken,
        refreshToken: data.refreshToken,
        expiresAt: Date.now() + data.expiresIn * 1000
      };
      storeTokens(tokens);

      const user: User = data.user;
      setState({ user, isAuthenticated: true, isLoading: false });
      setCurrentUser(user);
    } catch (error) {
      setState({ error: (error as Error).message, isLoading: false });
      throw error;
    }
  };

  // Sign up function
  const signUp = async (email: string, password: string, name: string) => {
    setState({ isLoading: true, error: null });
    try {
      const response = await fetch(`${API_BASE_URL}/auth/register`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ email, password, name }),
        credentials: 'include'
      });

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Registration failed');
      }

      const data = await response.json();
      const tokens: AuthTokens = {
        accessToken: data.accessToken,
        refreshToken: data.refreshToken,
        expiresAt: Date.now() + data.expiresIn * 1000
      };
      storeTokens(tokens);

      const user: User = data.user;
      setState({ user, isAuthenticated: true, isLoading: false });
      setCurrentUser(user);
    } catch (error) {
      setState({ error: (error as Error).message, isLoading: false });
      throw error;
    }
  };

  // Sign out function
  const signOut = async () => {
    try {
      const accessToken = await getValidAccessToken();
      if (accessToken) {
        await fetch(`${API_BASE_URL}/auth/logout`, {
          method: 'POST',
          headers: {
            'Authorization': `Bearer ${accessToken}`,
            'Content-Type': 'application/json'
          },
          credentials: 'include'
        });
      }
    } catch (error) {
      console.error('Logout error:', error);
    } finally {
      clearTokens();
      setState({ user: null, isAuthenticated: false, isLoading: false });
      setCurrentUser(null);
    }
  };

  // Forgot password
  const forgotPassword = async (email: string) => {
    setState({ isLoading: true, error: null });
    try {
      const response = await fetch(`${API_BASE_URL}/auth/forgot-password`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ email }),
        credentials: 'include'
      });

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Failed to send reset email');
      }
      setState({ isLoading: false });
    } catch (error) {
      setState({ error: (error as Error).message, isLoading: false });
      throw error;
    }
  };

  // Reset password
  const resetPassword = async (token: string, newPassword: string) => {
    setState({ isLoading: true, error: null });
    try {
      const response = await fetch(`${API_BASE_URL}/auth/reset-password`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ token, newPassword }),
        credentials: 'include'
      });

      if (!response.ok) {
        const errorData = await response.json();
        throw new Error(errorData.message || 'Failed to reset password');
      }
      setState({ isLoading: false });
    } catch (error) {
      setState({ error: (error as Error).message, isLoading: false });
      throw error;
    }
  };

  // Initialize auth state
  createEffect(async () => {
    const tokens = getStoredTokens();
    if (tokens) {
      // Check if token is still valid
      if (Date.now() < tokens.expiresAt) {
        // Try to refresh if close to expiration, or fetch user profile
        try {
          const accessToken = await getValidAccessToken();
          if (accessToken) {
            // Fetch user profile
            const userResponse = await fetch(`${API_BASE_URL}/auth/profile`, {
              headers: {
                'Authorization': `Bearer ${accessToken}`
              },
              credentials: 'include'
            });
            if (userResponse.ok) {
              const user = await userResponse.json();
              setState({ user, isAuthenticated: true, isLoading: false });
              setCurrentUser(user);
              return;
            }
          }
        } catch {
          // If fetching user fails, clear state
        }
      }
      // If we get here, token is invalid/expired
      clearTokens();
    }
    setState({ isLoading: false });
  });

  // Set up periodic token refresh
  createEffect(() => {
    if (!state.isAuthenticated) return;

    const interval = setInterval(async () => {
      await refreshToken();
    }, 60000); // Check every minute

    return () => clearInterval(interval);
  });

  const value: AuthContextType = {
    state,
    setState,
    currentUser,
    signIn,
    signUp,
    signOut,
    refreshToken,
    forgotPassword,
    resetPassword
  };

  return <AuthContext.Provider value={value}>{props.children}</AuthContext.Provider>;
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}

// Hook to get authenticated user, redirects if not authenticated
export function useRequiredAuth() {
  const { state, signOut } = useAuth();
  const navigate = useNavigate();

  createEffect(() => {
    if (!state.isLoading && !state.isAuthenticated) {
      navigate('/sign-in');
    }
  });

  return useAuth();
}

// Import useNavigate - this is a workaround, in actual implementation import from @solidjs/router
function useNavigate() {
  return (path: string) => {
    window.location.href = path;
  };
}