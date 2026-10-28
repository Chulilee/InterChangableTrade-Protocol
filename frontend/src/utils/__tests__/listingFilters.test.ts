import { describe, it, expect } from 'vitest';
import {
  DEFAULT_FILTERS,
  clampPriceRange,
  countActiveFilters,
  filtersToQueryString,
  filtersToSearchParams,
  parseFiltersFromSearch,
  toListingsQuery,
  toggleValue
} from '../listingFilters';

describe('parseFiltersFromSearch', () => {
  it('returns defaults for an empty search string', () => {
    expect(parseFiltersFromSearch('')).toEqual(DEFAULT_FILTERS);
    expect(parseFiltersFromSearch('?')).toEqual(DEFAULT_FILTERS);
  });

  it('parses every supported parameter', () => {
    const filters = parseFiltersFromSearch(
      '?q=camera&categories=electronics,photo&tags=rare,vintage&minPrice=10&maxPrice=250&sort=price-asc&page=3'
    );

    expect(filters).toEqual({
      search: 'camera',
      categories: ['electronics', 'photo'],
      tags: ['rare', 'vintage'],
      minPrice: 10,
      maxPrice: 250,
      sortBy: 'price',
      sortOrder: 'asc',
      page: 3
    });
  });

  it('accepts a search string without the leading question mark', () => {
    expect(parseFiltersFromSearch('q=lamp').search).toBe('lamp');
  });

  it('ignores unknown sort values and invalid page numbers', () => {
    expect(parseFiltersFromSearch('?sort=most-liked').sortBy).toBe('date');
    expect(parseFiltersFromSearch('?sort=most-liked').sortOrder).toBe('desc');
    expect(parseFiltersFromSearch('?page=0').page).toBe(1);
    expect(parseFiltersFromSearch('?page=-2').page).toBe(1);
    expect(parseFiltersFromSearch('?page=abc').page).toBe(1);
  });

  it('drops blank and duplicate list entries', () => {
    const filters = parseFiltersFromSearch('?categories=a,,a,b,&tags=%20x%20');
    expect(filters.categories).toEqual(['a', 'b']);
    expect(filters.tags).toEqual(['x']);
  });

  it('ignores negative and non-numeric prices', () => {
    expect(parseFiltersFromSearch('?minPrice=-5').minPrice).toBe(null);
    expect(parseFiltersFromSearch('?maxPrice=nope').maxPrice).toBe(null);
  });

  it('swaps inverted price bounds', () => {
    const filters = parseFiltersFromSearch('?minPrice=200&maxPrice=50');
    expect(filters.minPrice).toBe(50);
    expect(filters.maxPrice).toBe(200);
  });
});

describe('filtersToSearchParams', () => {
  it('omits default values', () => {
    expect(filtersToSearchParams(DEFAULT_FILTERS).toString()).toBe('');
    expect(filtersToQueryString(DEFAULT_FILTERS)).toBe('');
  });

  it('serializes active filters with deterministic ordering', () => {
    const params = filtersToSearchParams({
      ...DEFAULT_FILTERS,
      search: 'camera',
      categories: ['photo', 'electronics'],
      tags: ['vintage', 'rare'],
      minPrice: 10,
      maxPrice: 250,
      sortBy: 'price',
      sortOrder: 'asc',
      page: 2
    });

    expect(params.get('q')).toBe('camera');
    expect(params.get('categories')).toBe('electronics,photo');
    expect(params.get('tags')).toBe('rare,vintage');
    expect(params.get('minPrice')).toBe('10');
    expect(params.get('maxPrice')).toBe('250');
    expect(params.get('sort')).toBe('price-asc');
    expect(params.get('page')).toBe('2');
  });

  it('round-trips through parseFiltersFromSearch', () => {
    const original = {
      ...DEFAULT_FILTERS,
      search: 'lamp',
      categories: ['home'],
      tags: ['brass'],
      minPrice: 5,
      maxPrice: 90,
      sortBy: 'price' as const,
      sortOrder: 'desc' as const,
      page: 4
    };
    expect(parseFiltersFromSearch(filtersToQueryString(original))).toEqual(original);
  });
});

describe('toListingsQuery', () => {
  it('maps filters to API query params', () => {
    const query = toListingsQuery(
      {
        ...DEFAULT_FILTERS,
        search: 'camera',
        categories: ['electronics'],
        tags: ['rare'],
        minPrice: 0,
        maxPrice: 100,
        page: 2
      },
      12
    );

    expect(query).toEqual({
      page: 2,
      limit: 12,
      sortBy: 'date',
      sortOrder: 'desc',
      search: 'camera',
      categories: ['electronics'],
      tags: ['rare'],
      minPrice: 0,
      maxPrice: 100
    });
  });

  it('omits empty optional fields', () => {
    const query = toListingsQuery(DEFAULT_FILTERS, 12);
    expect(query.search).toBeUndefined();
    expect(query.categories).toBeUndefined();
    expect(query.tags).toBeUndefined();
    expect(query.minPrice).toBeUndefined();
    expect(query.maxPrice).toBeUndefined();
  });
});

describe('toggleValue', () => {
  it('adds missing values and removes present ones', () => {
    expect(toggleValue(['a'], 'b')).toEqual(['a', 'b']);
    expect(toggleValue(['a', 'b'], 'a')).toEqual(['b']);
    expect(toggleValue([], 'a')).toEqual(['a']);
  });
});

describe('countActiveFilters', () => {
  it('counts each active facet', () => {
    expect(countActiveFilters(DEFAULT_FILTERS)).toBe(0);
    expect(
      countActiveFilters({
        ...DEFAULT_FILTERS,
        search: 'x',
        categories: ['a'],
        tags: ['t1', 't2'],
        minPrice: 1,
        maxPrice: 2
      })
    ).toBe(6);
  });
});

describe('clampPriceRange', () => {
  it('coerces bounds into the ceiling and keeps min <= max', () => {
    expect(clampPriceRange(null, null, 1000)).toEqual({ min: 0, max: 1000 });
    expect(clampPriceRange(-5, 2000, 1000)).toEqual({ min: 0, max: 1000 });
    expect(clampPriceRange(200, 50, 1000)).toEqual({ min: 50, max: 200 });
  });
});
