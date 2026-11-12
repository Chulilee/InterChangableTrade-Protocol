import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook } from '@solidjs/testing-library';
import { AuthProvider, useAuth } from '../AuthContext';

// Mock fetch
global.fetch = vi.fn();

describe('AuthContext', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    sessionStorage.clear();
  });

  it('initializes with unauthenticated state when no session can be refreshed', async () => {
    (fetch as vi.Mock).mockResolvedValueOnce({ ok: false, status: 401 });

    const { result } = renderHook(() => useAuth(), {
      wrapper: AuthProvider
    });

    await vi.waitFor(() => expect(result.state.isLoading).toBe(false));
    expect(result.state.isAuthenticated).toBe(false);
    expect(result.state.user).toBeNull();
    expect(localStorage.getItem('auth_tokens')).toBeNull();
  });

  it('keeps access tokens in memory only on successful login (no localStorage persistence)', async () => {
    const mockUser = {
      id: '1',
      email: 'test@example.com',
      name: 'Test User',
      createdAt: '2024-01-01'
    };

    (fetch as vi.Mock)
      // init refresh attempt (no in-memory token)
      .mockResolvedValueOnce({ ok: false, status: 401 })
      // login
      .mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            accessToken: 'mock-access-token',
            expiresIn: 3600,
            user: mockUser
          })
      });

    const { result } = renderHook(() => useAuth(), {
      wrapper: AuthProvider
    });

    await vi.waitFor(() => expect(result.state.isLoading).toBe(false));
    await result.signIn('test@example.com', 'password123');

    expect(result.state.isAuthenticated).toBe(true);
    expect(result.state.user).toEqual(mockUser);

    // Sensitive tokens must not be stored in browser storage.
    expect(localStorage.getItem('auth_tokens')).toBeNull();
    expect(localStorage.getItem('accessToken')).toBeNull();
    expect(localStorage.getItem('refreshToken')).toBeNull();
  });

  it('clears in-memory session on logout without using localStorage tokens', async () => {
    const mockUser = {
      id: '1',
      email: 'test@example.com',
      name: 'Test User',
      createdAt: '2024-01-01'
    };

    (fetch as vi.Mock)
      // init refresh attempt
      .mockResolvedValueOnce({ ok: false, status: 401 })
      // login
      .mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            accessToken: 'mock-access-token',
            expiresIn: 3600,
            user: mockUser
          })
      })
      // logout
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({}) });

    const { result } = renderHook(() => useAuth(), {
      wrapper: AuthProvider
    });

    await vi.waitFor(() => expect(result.state.isLoading).toBe(false));
    await result.signIn('test@example.com', 'password123');
    expect(result.state.isAuthenticated).toBe(true);
    expect(localStorage.getItem('auth_tokens')).toBeNull();

    await result.signOut();

    expect(localStorage.getItem('auth_tokens')).toBeNull();
    expect(result.state.isAuthenticated).toBe(false);
    expect(result.state.user).toBeNull();
  });
});
