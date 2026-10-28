import { createSignal, createResource, createMemo, createEffect, onMount, onCleanup, For, Show } from 'solid-js';
import { useNavigate, useLocation } from '@solidjs/router';
import { ListingCard } from './ListingCard';
import { SearchBar } from './SearchBar';
import { FilterPanel } from './FilterPanel';
import { fetchListings, deleteListing } from '../../api/listings';
import { useApi } from '../../hooks/useApi';
import { useAuth } from '../../contexts/AuthContext';
import type { Listing, ListingSortBy, SortOrder } from '../../types/listing';
import {
  DEFAULT_FILTERS,
  filtersToQueryString,
  parseFiltersFromSearch,
  toListingsQuery,
  type ListingFilters
} from '../../utils/listingFilters';

const PAGE_SIZE = 12;

type SortValue = `${ListingSortBy}-${SortOrder}`;

const SORT_OPTIONS: { value: SortValue; label: string }[] = [
  { value: 'date-desc', label: 'Newest first' },
  { value: 'date-asc', label: 'Oldest first' },
  { value: 'price-asc', label: 'Price: Low to High' },
  { value: 'price-desc', label: 'Price: High to Low' }
];

const MIN_PRICE_CEILING = 1000;

/**
 * Paginated, searchable, filterable and sortable list of marketplace listings.
 * Renders `ListingCard` for each item and lets the owner of a listing
 * edit/delete it inline.
 *
 * Search, category/tag filters, the price range, sorting and the current page
 * are mirrored into the URL (see `utils/listingFilters.ts`) so filtered views
 * can be shared or restored on reload.
 */
export function ListingList() {
  const navigate = useNavigate();
  const location = useLocation();
  const { currentUser } = useAuth();
  const { apiRequest } = useApi();

  const [filters, setFilters] = createSignal<ListingFilters>(
    parseFiltersFromSearch(location.search)
  );
  const [deleteError, setDeleteError] = createSignal<string | null>(null);

  // Facet options accumulate as pages are visited so filtering by one category
  // does not hide the other available categories/tags.
  const [seenCategories, setSeenCategories] = createSignal<string[]>([]);
  const [seenTags, setSeenTags] = createSignal<string[]>([]);

  const queryKey = createMemo(() => toListingsQuery(filters(), PAGE_SIZE));
  const [listingsResource, { refetch }] = createResource(queryKey, fetchListings);

  const totalPages = () => listingsResource()?.totalPages ?? 1;

  createEffect(() => {
    const items = listingsResource()?.items ?? [];
    if (items.length === 0) return;

    const categories = new Set(seenCategories());
    const tags = new Set(seenTags());
    let changed = false;

    for (const item of items) {
      if (item.category && !categories.has(item.category)) {
        categories.add(item.category);
        changed = true;
      }
      for (const tag of item.tags ?? []) {
        if (!tags.has(tag)) {
          tags.add(tag);
          changed = true;
        }
      }
    }

    if (changed) {
      setSeenCategories(Array.from(categories).sort());
      setSeenTags(Array.from(tags).sort());
    }
  });

  const priceCeiling = createMemo(() => {
    const highest = (listingsResource()?.items ?? []).reduce(
      (max, listing) => Math.max(max, listing.price),
      0
    );
    return Math.max(MIN_PRICE_CEILING, Math.ceil(highest / 100) * 100);
  });

  const suggestions = createMemo(() => {
    const terms = new Set<string>();
    for (const item of listingsResource()?.items ?? []) {
      if (item.title) terms.add(item.title);
      if (item.category) terms.add(item.category);
      for (const tag of item.tags ?? []) terms.add(tag);
    }
    return Array.from(terms).slice(0, 8);
  });

  // Mirror the filter state into the URL for shareable links. `replaceState`
  // avoids polluting browser history for every keystroke/filter tweak.
  createEffect(() => {
    if (typeof window === 'undefined') return;
    const target = `${location.pathname}${filtersToQueryString(filters())}`;
    const current = `${window.location.pathname}${window.location.search}`;
    if (current !== target) {
      window.history.replaceState(null, '', target);
    }
  });

  onMount(() => {
    if (typeof window === 'undefined') return;
    const handlePopState = () => setFilters(parseFiltersFromSearch(window.location.search));
    window.addEventListener('popstate', handlePopState);
    onCleanup(() => window.removeEventListener('popstate', handlePopState));
  });

  /** Updates filters and returns to the first page. */
  const updateFilters = (partial: Partial<ListingFilters>) => {
    setFilters((previous) => ({ ...previous, ...partial, page: 1 }));
  };

  const goToPage = (page: number) => setFilters((previous) => ({ ...previous, page }));

  const resetFilters = () =>
    setFilters({ ...DEFAULT_FILTERS, categories: [], tags: [] });

  const handleSortChange = (value: string) => {
    const [by, order] = value.split('-') as [ListingSortBy, SortOrder];
    updateFilters({ sortBy: by, sortOrder: order });
  };

  const handleEdit = (listing: Listing) => navigate(`/listings/${listing.id}/edit`);

  const handleDelete = async (listing: Listing) => {
    setDeleteError(null);
    try {
      await deleteListing(listing.id, apiRequest);
      await refetch();
    } catch (error) {
      setDeleteError((error as Error).message);
    }
  };

  const resultAnnouncement = () => {
    if (listingsResource.loading) return 'Loading listings\u2026';
    if (listingsResource.error) return 'Failed to load listings';
    const total = listingsResource()?.total ?? 0;
    return `${total} listing${total === 1 ? '' : 's'} found`;
  };

  return (
    <div class="max-w-6xl mx-auto px-4 py-8">
      <div class="flex flex-col sm:flex-row sm:items-center gap-4 mb-6">
        <SearchBar
          value={filters().search}
          onDebouncedChange={(value) => updateFilters({ search: value })}
          suggestions={suggestions()}
        />

        <div>
          <label for="listing-sort" class="sr-only">
            Sort listings
          </label>
          <select
            id="listing-sort"
            class="border rounded px-3 py-2 text-sm"
            value={`${filters().sortBy}-${filters().sortOrder}`}
            onChange={(event) => handleSortChange(event.currentTarget.value)}
          >
            <For each={SORT_OPTIONS}>{(option) => <option value={option.value}>{option.label}</option>}</For>
          </select>
        </div>
      </div>

      <Show when={deleteError()}>
        <div role="alert" class="text-red-500 text-sm bg-red-50 p-2 rounded mb-4">
          {deleteError()}
        </div>
      </Show>

      <div class="grid grid-cols-1 lg:grid-cols-[260px_1fr] gap-6 items-start">
        <FilterPanel
          filters={filters()}
          availableCategories={seenCategories()}
          availableTags={seenTags()}
          priceCeiling={priceCeiling()}
          onCategoriesChange={(categories) => updateFilters({ categories })}
          onTagsChange={(tags) => updateFilters({ tags })}
          onPriceChange={(minPrice, maxPrice) => updateFilters({ minPrice, maxPrice })}
          onReset={resetFilters}
        />

        <section aria-label="Listings" aria-busy={listingsResource.loading} id="listing-results">
          <p class="sr-only" aria-live="polite">
            {resultAnnouncement()}
          </p>

          <Show
            when={!listingsResource.loading}
            fallback={<p class="text-center text-gray-500 py-12">Loading listings...</p>}
          >
            <Show
              when={!listingsResource.error}
              fallback={
                <div class="text-center py-12">
                  <p class="text-red-600">
                    {(listingsResource.error as Error)?.message || 'Failed to load listings'}
                  </p>
                  <button type="button" class="mt-2 text-sm text-blue-600 underline" onClick={() => refetch()}>
                    Retry
                  </button>
                </div>
              }
            >
              <Show
                when={(listingsResource()?.items.length ?? 0) > 0}
                fallback={<p class="text-center text-gray-500 py-12">No listings found.</p>}
              >
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-6">
                  <For each={listingsResource()?.items}>
                    {(listing) => (
                      <ListingCard
                        listing={listing}
                        canManage={currentUser()?.id === listing.sellerId}
                        onEdit={handleEdit}
                        onDelete={handleDelete}
                      />
                    )}
                  </For>
                </div>

                <div class="flex items-center justify-center gap-4 mt-8">
                  <button
                    type="button"
                    class="px-3 py-1 border rounded disabled:opacity-40"
                    disabled={filters().page <= 1}
                    onClick={() => goToPage(Math.max(1, filters().page - 1))}
                  >
                    Previous
                  </button>
                  <span class="text-sm text-gray-600">
                    Page {filters().page} of {totalPages()}
                  </span>
                  <button
                    type="button"
                    class="px-3 py-1 border rounded disabled:opacity-40"
                    disabled={filters().page >= totalPages()}
                    onClick={() => goToPage(Math.min(totalPages(), filters().page + 1))}
                  >
                    Next
                  </button>
                </div>
              </Show>
            </Show>
          </Show>
        </section>
      </div>
    </div>
  );
}
