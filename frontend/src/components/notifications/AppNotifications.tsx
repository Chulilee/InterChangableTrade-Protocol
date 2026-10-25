// Notification feedback for the app (part of the Profile & Settings feature,
// issue #28): profile pages surface success/error feedback through this
// shared in-app notification host, which is gated by the user's
// `inAppNotifications` preference — that is how notification toggles affect
// app behavior.
//
// RFC-style tradeoff: this ships as a small, self-contained replacement for
// suppressing feedback entirely on sites without a notification system yet,
// rather than a full toast library. `shouldShowToast` is pure and extracted so
// gating is unit-testable.

import { createContext, useContext, createSignal, JSX, Accessor } from 'solid-js';
import type { UserPreferences } from '../../types/user';

export type ToastKind = 'success' | 'error';

export interface PendingToast {
  id: number;
  kind: ToastKind;
  message: string;
}

/**
 * Pure gating rule: informational (success) toasts only appear when the user
 * has not disabled in-app notifications; errors always surface so problems
 * are never silently swallowed.
 */
export function shouldShowToast(kind: ToastKind, preferences: UserPreferences): boolean {
  return kind === 'error' || preferences.inAppNotifications;
}

interface AppNotificationsContextType {
  toasts: Accessor<PendingToast[]>;
  notify: (message: string, kind?: ToastKind) => void;
  dismiss: (id: number) => void;
}

const AppNotificationsContext = createContext<AppNotificationsContextType>();

export function AppNotificationsProvider(props: { children: JSX.Element }) {
  const { preferences } = usePreferences();

  const [toasts, setToasts] = createSignal<PendingToast[]>([]);
  let nextId = 1;

  const dismiss = (id: number) => {
    setToasts((current) => current.filter((toast) => toast.id !== id));
  };

  const notify: AppNotificationsContextType['notify'] = (message, kind = 'success') => {
    if (!shouldShowToast(kind, preferences())) {
      return;
    }
    const id = nextId++;
    setToasts((current) => [...current, { id, kind, message }]);
    // Auto-dismiss success feedback; errors stay until acknowledged.
    if (kind === 'success') {
      setTimeout(() => dismiss(id), 4000);
    }
  };

  const value: AppNotificationsContextType = { toasts, notify, dismiss };

  return (
    <>
      <AppNotificationsContext.Provider value={value}>
        {props.children}
      </AppNotificationsContext.Provider>
      <ToastHost toasts={toasts} onDismiss={dismiss} />
    </>
  );
}

export function useAppNotifications() {
  const context = useContext(AppNotificationsContext);
  if (!context) {
    throw new Error('useAppNotifications must be used within an AppNotificationsProvider');
  }
  return context;
}

function ToastHost(props: {
  toasts: Accessor<PendingToast[]>;
  onDismiss: (id: number) => void;
}) {
  return (
    <div class="fixed bottom-4 right-4 z-50 flex flex-col gap-2" role="status" aria-live="polite">
      {props.toasts().map((toast) => (
        <div
          class={
            'px-4 py-3 rounded shadow-md text-sm ' +
            (toast.kind === 'error'
              ? 'bg-red-600 text-white'
              : 'bg-green-600 text-white')
          }
        >
          <span>{toast.message}</span>
          <button
            type="button"
            class="ml-3 font-bold"
            onClick={() => props.onDismiss(toast.id)}
          >
            ×
          </button>
        </div>
      ))}
    </div>
  );
}
