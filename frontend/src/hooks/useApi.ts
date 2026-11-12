import { useAuth } from '../contexts/AuthContext';
import { getCsrfHeaders } from '../utils/secureStorage';

export function useApi() {
  const { getAccessToken, refreshToken } = useAuth();

  const apiRequest = async (
    url: string,
    options: RequestInit = {}
  ): Promise<Response> => {
    let accessToken = await getAccessToken();
    if (!accessToken) {
      throw new Error('No authentication token available');
    }

    // Add authorization and CSRF headers. Content-Type stays unset for FormData
    // bodies (e.g. avatar uploads): the browser must supply the multipart
    // boundary, and forcing application/json here would corrupt the upload.
    const headers = new Headers(options.headers);
    headers.set('Authorization', `Bearer ${accessToken}`);
    if (!(options.body instanceof FormData)) {
      headers.set('Content-Type', 'application/json');
    }
    const csrfHeaders = getCsrfHeaders();
    for (const [key, value] of Object.entries(csrfHeaders)) {
      headers.set(key, value);
    }

    const response = await fetch(url, {
      ...options,
      headers,
      credentials: 'include'
    });

    // If we get a 401, try to refresh token once and retry
    if (response.status === 401) {
      const refreshed = await refreshToken();
      if (refreshed) {
        accessToken = await getAccessToken();
        if (accessToken) {
          headers.set('Authorization', `Bearer ${accessToken}`);
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
