// Preferences context for the Profile & Settings feature (issue #28).
//
// Owns the user's saved notification preferences and exposes them to any
// consuming component (e.g. the in-app notification host gates success toasts
// on `inAppNotifications`). A signed-in user's preferences are loaded from the
// user settings endpoint; they fall back to localStorage so the app still
// behaves on later visits even before the endpoint responds.

import {
  createContext,
  useContext,
  createEffect,
  createSignal,
  JSX,
  Accessor,
} from 'solid-js';
import { useAuth } from './AuthContext';
import { useApi } from '../hooks/useApi';
import { fetchMyPreferences, updateMyPreferences } from '../api/users';
import { DEFAULT_PREFERENCES, type UserPreferences } from '../types/user';

const PREFERENCES_STORAGE_KEY = 'user_preferences';

interface PreferencesContextType {
  /** Current preferences (defaults until they load). */
  preferences: Accessor<UserPreferences>;
  isLoading: Accessor<boolean>;
  error: Accessor<string | null>;
  /** Reloads the saved preferences from the settings endpoint. */
  reload: () => Promise<void>;
  /** Persists new preferences via the settings endpoint and updates the store. */
  save: (next: UserPreferences) => Promise<void>;
}

const PreferencesContext = createContext<PreferencesContextType>();

function readStoredPreferences(): UserPreferences {
  try {
    const raw = localStorage.getItem(PREFERENCES_STORAGE_KEY);
    if (raw) return { ...DEFAULT_PREFERENCES, ...JSON.parse(raw) };
  } catch {
    // Corrupt or unavailable storage - fall back to defaults.
  }
  return DEFAULT_PREFERENCES;
}

function storePreferencesLocally(preferences: UserPreferences): void {
  try {
    localStorage.setItem(PREFERENCES_STORAGE_KEY, JSON.stringify(preferences));
  } catch {
    // Storage may be unavailable (private mode) - preferences are still saved
    // server-side by the caller.
  }
}

export function PreferencesProvider(props: { children: JSX.Element }) {
  const { state } = useAuth();

  const [preferences, setPreferences] = createSignal<UserPreferences>(readStoredPreferences());
  const [isLoading, setIsLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const load = async () => {
    const { apiRequest } = useApi();
    setIsLoading(true);
    setError(null);
    try {
      const saved = await fetchMyPreferences(apiRequest);
      setPreferences({ ...DEFAULT_PREFERENCES, ...saved });
      storePreferencesLocally(saved);
    } catch (err) {
      setError((err as Error).message);
    } finally {
      setIsLoading(false);
    }
  };

  const reload = async () => {
    await load();
  };

  const save = async (next: UserPreferences) => {
    const { apiRequest } = useApi();
    const saved = await updateMyPreferences(next, apiRequest);
    setPreferences({ ...DEFAULT_PREFERENCES, ...saved });
    storePreferencesLocally(saved);
  };

  // Load saved preferences once the user signs in.
  createEffect(() => {
    if (state.isAuthenticated) {
      void load();
    }
  });

  const value: PreferencesContextType = {
    preferences,
    isLoading,
    error,
    reload,
    save
  };

  return <PreferencesContext.Provider value={value}>{props.children}</PreferencesContext.Provider>;
}

export function usePreferences(): PreferencesContextType {
  const context = useContext(PreferencesContext);
  if (!context) {
    throw new Error('usePreferences must be used within a PreferencesProvider');
  }
  return context;
}
