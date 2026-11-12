import { createContext, useContext, createSignal, createEffect, Accessor } from 'solid-js';
import { createStore, Store, SetStoreFunction } from 'solid-js/store';
import { useNavigate } from '@solidjs/router';
import {
  getCsrfHeaders,
  purgeSensitiveBrowserStorage,
  secureTokenStorage,
} from '../utils/secureStorage';

// Types
export interface User {
  id: string;
  email: string;
  name: string;
  /** Role strings carried in the auth payload (e.g. ["admin"], issue #26). */
  roles?: string[];
  createdAt: string;
}

export interface AuthTokens {
  /** Kept in memory only (never persisted to localStorage/sessionStorage). */
  accessToken: string;
  /**
   * Refresh tokens are expected to be managed via httpOnly cookies set by the
   * backend and are not stored in JavaScript-accessible storage.
   */
  refreshToken?: string;
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
  /** Returns a valid in-memory access token, refreshing if needed. */
  getAccessToken: () => Promise<string | null>;
  forgotPassword: (email: string) => Promise<void>;
  resetPassword: (token: string, newPassword: string) => Promise<void>;
}

// API endpoints - configure these based on your backend
const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:3000/api';
const TOKEN_REFRESH_THRESHOLD = 5 * 60 * 1000; // Refresh 5 minutes before expiration

const AuthContext = createContext<AuthContextType>();

function clearSensitiveSessionState(
  setState: SetStoreFunction<AuthState>,
  setCurrentUser: (user: User | null) => void
) {
  secureTokenStorage.clear();
  purgeSensitiveBrowserStorage();
  setState({ isAuthenticated: false, user: null, isLoading: false });
  setCurrentUser(null);
}

export function AuthProvider(props: { children: any }) {
  const [state, setState] = createStore<AuthState>({
    user: null,
    isAuthenticated: false,
    isLoading: true,
    error: null
  });

  const [currentUser, setCurrentUser] = createSignal<User | null>(null);

  const storeTokens = (tokens: AuthTokens) => {
    // Access token is kept in memory only; never persist sensitive tokens.
    secureTokenStorage.setTokens({
      accessToken: tokens.accessToken,
      expiresAt: tokens.expiresAt,
    });
    purgeSensitiveBrowserStorage();
  };

  const clearTokens = () => {
    secureTokenStorage.clear();
    purgeSensitiveBrowserStorage();
  };

  // Token refresh logic (refresh token is expected in an httpOnly cookie).
  const refreshToken = async (): Promise<boolean> => {
    try {
      const response = await fetch(`${API_BASE_URL}/auth/refresh`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...getCsrfHeaders(),
        },
        credentials: 'include'
      });

      if (!response.ok) {
        throw new Error('Token refresh failed');
      }

      const data = await response.json();
      const accessToken = data.accessToken as string | undefined;
      const expiresIn = Number(data.expiresIn ?? 0);
      if (!accessToken || !Number.isFinite(expiresIn) || expiresIn <= 0) {
        throw new Error('Invalid refresh response');
      }

      const newTokens: AuthTokens = {
        accessToken,
        expiresAt: Date.now() + expiresIn * 1000
      };
      storeTokens(newTokens);
      return true;
    } catch (error) {
      console.error('Token refresh error:', error);
      clearSensitiveSessionState(setState, setCurrentUser);
      return false;
    }
  };

  // Get valid access token (refreshes if needed).
  const getAccessToken = async (): Promise<string | null> => {
    const accessToken = secureTokenStorage.getAccessToken();
    const expiresAt = secureTokenStorage.getExpiresAt();
    if (!accessToken || !expiresAt) return null;

    if (Date.now() >= expiresAt - TOKEN_REFRESH_THRESHOLD) {
      const refreshed = await refreshToken();
      if (!refreshed) return null;
      return secureTokenStorage.getAccessToken();
    }

    return accessToken;
  };

  // Sign in function
  const signIn = async (email: string, password: string) => {
    setState({ isLoading: true, error: null });
    try {
      const response = await fetch(`${API_BASE_URL}/auth/login`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          ...getCsrfHeaders(),
        },
        body: JSON.stringify({ email, password }),
        credentials: 'include'
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error((errorData as any).message || 'Login failed');
      }

      const data = await response.json();
      const tokens: AuthTokens = {
        accessToken: data.accessToken,
        expiresAt: Date.now() + Number(data.expiresIn ?? 0) * 1000
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
          ...getCsrfHeaders(),
        },
        body: JSON.stringify({ email, password, name }),
        credentials: 'include'
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error((errorData as any).message || 'Registration failed');
      }

      const data = await response.json();
      const tokens: AuthTokens = {
        accessToken: data.accessToken,
        expiresAt: Date.now() + Number(data.expiresIn ?? 0) * 1000
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
      const accessToken = await getAccessToken();
      if (accessToken) {
        await fetch(`${API_BASE_URL}/auth/logout`, {
          method: 'POST',
          headers: {
            'Authorization': `Bearer ${accessToken}`,
            'Content-Type': 'application/json',
            ...getCsrfHeaders(),
          },
          credentials: 'include'
        });
      } else {
        // Best-effort logout to clear httpOnly refresh cookie server-side.
        await fetch(`${API_BASE_URL}/auth/logout`, {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
            ...getCsrfHeaders(),
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
          ...getCsrfHeaders(),
        },
        body: JSON.stringify({ email }),
        credentials: 'include'
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error((errorData as any).message || 'Failed to send reset email');
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
          ...getCsrfHeaders(),
        },
        body: JSON.stringify({ token, newPassword }),
        credentials: 'include'
      });

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        throw new Error((errorData as any).message || 'Failed to reset password');
      }
      setState({ isLoading: false });
    } catch (error) {
      setState({ error: (error as Error).message, isLoading: false });
      throw error;
    }
  };

  // Initialize auth state.
  // Sensitive tokens are never persisted in localStorage; if we don't have an
  // in-memory access token (e.g. full page reload), attempt a cookie-based
  // refresh to rehydrate a session without exposing refresh tokens to JS.
  createEffect(async () => {
    purgeSensitiveBrowserStorage();
    const existing = secureTokenStorage.getAccessToken();
    if (!existing) {
      const refreshed = await refreshToken();
      if (!refreshed) {
        setState({ isLoading: false });
        return;
      }
    }

    try {
      const accessToken = await getAccessToken();
      if (accessToken) {
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
      // Fall through to clear state.
    }

    clearSensitiveSessionState(setState, setCurrentUser);
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
    getAccessToken,
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
  const auth = useAuth();
  const navigate = useNavigate();

  createEffect(() => {
    if (!auth.state.isLoading && !auth.state.isAuthenticated) {
      navigate('/sign-in');
    }
  });

  return auth;
}
