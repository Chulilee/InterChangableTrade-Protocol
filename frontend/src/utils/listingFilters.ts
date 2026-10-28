// Pure helpers for the listings search/filter state (issue #30).
//
// The filter state is the single source of truth for the listing screen. It is
// serialized into the URL so that a filtered view can be shared or restored on
// reload, and converted into the `ListingsQuery` consumed by the API layer.

import type { ListingSortBy, ListingsQuery, SortOrder } from '../types/listing';

/** Filter state for the listings screen. `null` price bounds mean "unbounded". */
export interface ListingFilters {
  search: string;
  categories: string[];
  tags: string[];
  minPrice: number | null;
  maxPrice: number | null;
  sortBy: ListingSortBy;
  sortOrder: SortOrder;
  page: number;
}

export const DEFAULT_FILTERS: ListingFilters = {
  search: '',
  categories: [],
  tags: [],
  minPrice: null,
  maxPrice: null,
  sortBy: 'date',
  sortOrder: 'desc',
  page: 1
};

/** All accepted `sort` URL values. */
const SORT_VALUES = ['date-asc', 'date-desc', 'price-asc', 'price-desc'] as const;

function parseList(value: string | null): string[] {
  if (!value) return [];
  return Array.from(new Set(value.split(',').map((entry) => entry.trim()).filter(Boolean)));
}

function parsePrice(value: string | null): number | null {
  if (value === null || value.trim() === '') return null;
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : null;
}

/**
 * Parses the filter state out of a location search string (with or without the
 * leading `?`). Invalid or unknown values fall back to the defaults instead of
 * throwing, so a hand-edited URL can never break the screen.
 */
export function parseFiltersFromSearch(search: string): ListingFilters {
  const params = new URLSearchParams(search.startsWith('?') ? search.slice(1) : search);

  const sort = params.get('sort') ?? '';
  const [sortBy, sortOrder] = (SORT_VALUES as readonly string[]).includes(sort)
    ? (sort.split('-') as [ListingSortBy, SortOrder])
    : [DEFAULT_FILTERS.sortBy, DEFAULT_FILTERS.sortOrder];

  let minPrice = parsePrice(params.get('minPrice'));
  let maxPrice = parsePrice(params.get('maxPrice'));
  if (minPrice !== null && maxPrice !== null && minPrice > maxPrice) {
    [minPrice, maxPrice] = [maxPrice, minPrice];
  }

  const page = Number(params.get('page') ?? '1');

  return {
    search: (params.get('q') ?? '').trim(),
    categories: parseList(params.get('categories')),
    tags: parseList(params.get('tags')),
    minPrice,
    maxPrice,
    sortBy,
    sortOrder,
    page: Number.isInteger(page) && page > 0 ? page : 1
  };
}

/** Serializes filter state into URL search params, omitting default values. */
export function filtersToSearchParams(filters: ListingFilters): URLSearchParams {
  const params = new URLSearchParams();
  if (filters.search) params.set('q', filters.search);
  if (filters.categories.length) params.set('categories', [...filters.categories].sort().join(','));
  if (filters.tags.length) params.set('tags', [...filters.tags].sort().join(','));
  if (filters.minPrice !== null) params.set('minPrice', String(filters.minPrice));
  if (filters.maxPrice !== null) params.set('maxPrice', String(filters.maxPrice));
  if (filters.sortBy !== DEFAULT_FILTERS.sortBy || filters.sortOrder !== DEFAULT_FILTERS.sortOrder) {
    params.set('sort', `${filters.sortBy}-${filters.sortOrder}`);
  }
  if (filters.page > 1) params.set('page', String(filters.page));
  return params;
}

/** Serializes filter state into a query string including the leading `?`. */
export function filtersToQueryString(filters: ListingFilters): string {
  const queryString = filtersToSearchParams(filters).toString();
  return queryString ? `?${queryString}` : '';
}

/** Converts filter state into the query object accepted by the listings API. */
export function toListingsQuery(filters: ListingFilters, limit: number): ListingsQuery {
  const query: ListingsQuery = {
    page: filters.page,
    limit,
    sortBy: filters.sortBy,
    sortOrder: filters.sortOrder
  };
  if (filters.search) query.search = filters.search;
  if (filters.categories.length) query.categories = [...filters.categories];
  if (filters.tags.length) query.tags = [...filters.tags];
  if (filters.minPrice !== null) query.minPrice = filters.minPrice;
  if (filters.maxPrice !== null) query.maxPrice = filters.maxPrice;
  return query;
}

/** Adds `value` to `values` if absent, removing it otherwise. */
export function toggleValue(values: readonly string[], value: string): string[] {
  return values.includes(value) ? values.filter((entry) => entry !== value) : [...values, value];
}

/** Number of active filter facets (search included), used for the reset badge. */
export function countActiveFilters(filters: ListingFilters): number {
  return (
    (filters.search ? 1 : 0) +
    filters.categories.length +
    filters.tags.length +
    (filters.minPrice !== null ? 1 : 0) +
    (filters.maxPrice !== null ? 1 : 0)
  );
}

/**
 * Coerces price bounds into `[0, ceiling]` and guarantees `min <= max`, so the
 * range sliders can never emit an inverted range.
 */
export function clampPriceRange(
  min: number | null,
  max: number | null,
  ceiling: number
): { min: number; max: number } {
  const low = Math.min(Math.max(min ?? 0, 0), ceiling);
  const high = Math.min(Math.max(max ?? ceiling, 0), ceiling);
  return low <= high ? { min: low, max: high } : { min: high, max: low };
}
