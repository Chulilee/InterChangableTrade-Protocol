import { useAuth } from '../contexts/AuthContext';

export function useApi() {
  const { refreshToken } = useAuth();
  
  const apiRequest = async (
    url: string,
    options: RequestInit = {}
  ): Promise<Response> => {
    const tokens = localStorage.getItem('auth_tokens');
    if (!tokens) {
      throw new Error('No authentication tokens found');
    }

    const parsedTokens = JSON.parse(tokens);
    let accessToken = parsedTokens.accessToken;
    
    // Check if token needs refreshing
    const TOKEN_REFRESH_THRESHOLD = 5 * 60 * 1000; // 5 minutes
    if (Date.now() >= parsedTokens.expiresAt - TOKEN_REFRESH_THRESHOLD) {
      const refreshed = await refreshToken();
      if (!refreshed) {
        throw new Error('Failed to refresh token');
      }
      const newTokens = localStorage.getItem('auth_tokens');
      if (newTokens) {
        accessToken = JSON.parse(newTokens).accessToken;
      }
    }

    // Add authorization header
    const headers = new Headers(options.headers);
    headers.set('Authorization', `Bearer ${accessToken}`);
    headers.set('Content-Type', 'application/json');

    const response = await fetch(url, {
      ...options,
      headers,
      credentials: 'include'
    });

    // If we get a 401, try to refresh token once and retry
    if (response.status === 401) {
      const refreshed = await refreshToken();
      if (refreshed) {
        const newTokens = localStorage.getItem('auth_tokens');
        if (newTokens) {
          const newAccessToken = JSON.parse(newTokens).accessToken;
          headers.set('Authorization', `Bearer ${newAccessToken}`);
          return fetch(url, {
            ...options,
            headers,
            credentials: 'include'
          });
        }
      }
      throw new Error('Session expired. Please sign in again.');
    }

    return response;
  };

  return { apiRequest };
}