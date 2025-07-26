import { localStorageStore } from "@skeletonlabs/skeleton";
import { writable, derived } from "svelte/store";
import type { Writable } from "svelte/store";

export type StoredUser = {
  email?: string;
  userId?: number;
};
export const user: Writable<StoredUser> = localStorageStore("user", {});

// Toast notifications
export interface ToastNotification {
  id: string;
  message: string;
  type: 'success' | 'error' | 'warning' | 'info';
  duration?: number;
  dismissible?: boolean;
}

export const toasts = writable<ToastNotification[]>([]);

export function addToast(toast: Omit<ToastNotification, 'id'>) {
  const id = Math.random().toString(36).substring(2, 9);
  const newToast: ToastNotification = {
    id,
    duration: 5000,
    dismissible: true,
    ...toast
  };
  
  toasts.update(t => [...t, newToast]);
  
  // Auto-remove toast after duration (browser-only)
  if (typeof window !== 'undefined' && newToast.duration && newToast.duration > 0) {
    setTimeout(() => {
      removeToast(id);
    }, newToast.duration);
  }
  
  return id;
}

export function removeToast(id: string) {
  toasts.update(t => t.filter(toast => toast.id !== id));
}

export function clearAllToasts() {
  toasts.set([]);
}

// Application state
export const appState = writable({
  isOnline: typeof navigator !== 'undefined' ? navigator.onLine : true,
  isLoading: false,
  lastActivity: Date.now()
});

// Derived states
export const isLoggedIn = derived(user, $user => !!$user.userId);

// Browser-only code
if (typeof window !== 'undefined') {
  // Update online status
  window.addEventListener('online', () => {
    appState.update(state => ({ ...state, isOnline: true }));
    addToast({ message: 'Connection restored', type: 'success' });
  });

  window.addEventListener('offline', () => {
    appState.update(state => ({ ...state, isOnline: false }));
    addToast({ message: 'You are currently offline', type: 'warning', duration: 0 });
  });

  // Track user activity for session management
  function updateActivity() {
    appState.update(state => ({ ...state, lastActivity: Date.now() }));
  }

  ['mousedown', 'mousemove', 'keypress', 'scroll', 'touchstart'].forEach(event => {
    document.addEventListener(event, updateActivity, { passive: true });
  });
}