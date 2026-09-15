// API integration for Marketplace Listings (issue #25).
//
// Follows the same conventions as AuthContext.tsx: a `VITE_API_URL` env var
// (defaulting to the local dev proxy target) and plain `fetch` for
// unauthenticated reads. Authenticated writes accept the `apiRequest`
// function produced by `useApi()` so token attachment/refresh stays
// centralized in one place instead of being duplicated here.

import type { Listing, ListingInput, ListingsQuery, PaginatedListings } from '../types/listing';

const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:3000/api';

/** Matches the shape of `useApi().apiRequest`. */
export type AuthedRequest = (url: string, options?: RequestInit) => Promise<Response>;

async function throwForErrorResponse(response: Response, fallback: string): Promise<never> {
  let message = fallback;
  try {
    const data = await response.json();
    if (data && typeof data.message === 'string') {
      message = data.message;
    }
  } catch {
    // Response body wasn't JSON (or was empty) - stick with the fallback.
  }
  throw new Error(message);
}

/** Builds a query string from listing query params, omitting empty values. */
export function buildListingsQuery(query: ListingsQuery = {}): string {
  const params = new URLSearchParams();
  if (query.page) params.set('page', String(query.page));
  if (query.limit) params.set('limit', String(query.limit));
  if (query.search) params.set('search', query.search);
  if (query.sortBy) params.set('sortBy', query.sortBy);
  if (query.sortOrder) params.set('sortOrder', query.sortOrder);
  const qs = params.toString();
  return qs ? `?${qs}` : '';
}

/** Fetches a page of listings, with optional search + sort params. */
export async function fetchListings(query: ListingsQuery = {}): Promise<PaginatedListings> {
  const response = await fetch(`${API_BASE_URL}/listings${buildListingsQuery(query)}`, {
    credentials: 'include'
  });
  if (!response.ok) {
    await throwForErrorResponse(response, 'Failed to load listings');
  }
  return response.json();
}

/** Fetches a single listing by id. */
export async function fetchListing(id: string): Promise<Listing> {
  const response = await fetch(`${API_BASE_URL}/listings/${encodeURIComponent(id)}`, {
    credentials: 'include'
  });
  if (!response.ok) {
    await throwForErrorResponse(response, 'Failed to load listing');
  }
  return response.json();
}

/** Creates a listing. Requires an authenticated `apiRequest`. */
export async function createListing(input: ListingInput, apiRequest: AuthedRequest): Promise<Listing> {
  const response = await apiRequest(`${API_BASE_URL}/listings`, {
    method: 'POST',
    body: JSON.stringify(input)
  });
  if (!response.ok) {
    await throwForErrorResponse(response, 'Failed to create listing');
  }
  return response.json();
}

/** Updates a listing. Requires an authenticated `apiRequest`. */
export async function updateListing(
  id: string,
  input: Partial<ListingInput>,
  apiRequest: AuthedRequest
): Promise<Listing> {
  const response = await apiRequest(`${API_BASE_URL}/listings/${encodeURIComponent(id)}`, {
    method: 'PUT',
    body: JSON.stringify(input)
  });
  if (!response.ok) {
    await throwForErrorResponse(response, 'Failed to update listing');
  }
  return response.json();
}

/** Deletes a listing. Requires an authenticated `apiRequest`. */
export async function deleteListing(id: string, apiRequest: AuthedRequest): Promise<void> {
  const response = await apiRequest(`${API_BASE_URL}/listings/${encodeURIComponent(id)}`, {
    method: 'DELETE'
  });
  if (!response.ok) {
    await throwForErrorResponse(response, 'Failed to delete listing');
  }
}
