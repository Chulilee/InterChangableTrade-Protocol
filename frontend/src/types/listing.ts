// Types for the Marketplace Listings feature (issue #25).
//
// These mirror the shape returned by the backend REST API. The frontend in
// this repo talks to a remote API via `VITE_API_URL` (see AuthContext.tsx
// for the established pattern) - there is no listings backend in this repo,
// so these types describe the contract the frontend expects from it.

export interface Listing {
  id: string;
  title: string;
  description: string;
  /** Price in the marketplace's quote currency (e.g. USD). */
  price: number;
  /** Image URLs attached to the listing. */
  images: string[];
  sellerId: string;
  sellerName?: string;
  /** Marketplace category, e.g. "electronics" (optional for legacy listings). */
  category?: string;
  /** Free-form tags used by search/filtering (optional for legacy listings). */
  tags?: string[];
  createdAt: string;
  updatedAt: string;
}

/** Fields a user supplies when creating or editing a listing. */
export interface ListingInput {
  title: string;
  description: string;
  price: number;
  images: string[];
  category?: string;
  tags?: string[];
}

export type ListingSortBy = 'date' | 'price';
export type SortOrder = 'asc' | 'desc';

export interface ListingsQuery {
  page?: number;
  limit?: number;
  /** Free-text search across title/description. */
  search?: string;
  sortBy?: ListingSortBy;
  sortOrder?: SortOrder;
  /** Restrict results to any of these categories. */
  categories?: string[];
  /** Restrict results to listings carrying at least one of these tags. */
  tags?: string[];
  /** Inclusive lower price bound. */
  minPrice?: number;
  /** Inclusive upper price bound. */
  maxPrice?: number;
}

export interface PaginatedListings {
  items: Listing[];
  total: number;
  page: number;
  limit: number;
  totalPages: number;
}
