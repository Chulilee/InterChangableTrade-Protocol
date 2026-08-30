import { createSignal, createResource, createMemo, For, Show } from 'solid-js';
import { useNavigate } from '@solidjs/router';
import { ListingCard } from './ListingCard';
import { fetchListings, deleteListing } from '../../api/listings';
import { useApi } from '../../hooks/useApi';
import { useAuth } from '../../contexts/AuthContext';
import type { Listing, ListingSortBy, SortOrder } from '../../types/listing';

const PAGE_SIZE = 12;

type SortValue = `${ListingSortBy}-${SortOrder}`;

const SORT_OPTIONS: { value: SortValue; label: string }[] = [
  { value: 'date-desc', label: 'Newest first' },
  { value: 'date-asc', label: 'Oldest first' },
  { value: 'price-asc', label: 'Price: Low to High' },
  { value: 'price-desc', label: 'Price: High to Low' }
];

/**
 * Paginated, searchable, sortable list of marketplace listings. Renders
 * `ListingCard` for each item and lets the owner of a listing edit/delete it
 * inline.
 */
export function ListingList() {
  const navigate = useNavigate();
  const { currentUser } = useAuth();
  const { apiRequest } = useApi();

  const [page, setPage] = createSignal(1);
  const [search, setSearch] = createSignal('');
  const [searchInput, setSearchInput] = createSignal('');
  const [sortBy, setSortBy] = createSignal<ListingSortBy>('date');
  const [sortOrder, setSortOrder] = createSignal<SortOrder>('desc');
  const [deleteError, setDeleteError] = createSignal<string | null>(null);

  const queryKey = createMemo(() => ({
    page: page(),
    limit: PAGE_SIZE,
    search: search(),
    sortBy: sortBy(),
    sortOrder: sortOrder()
  }));

  const [listingsResource, { refetch }] = createResource(queryKey, fetchListings);

  const totalPages = () => listingsResource()?.totalPages ?? 1;

  const handleSearchSubmit = (e: SubmitEvent) => {
    e.preventDefault();
    setPage(1);
    setSearch(searchInput().trim());
  };

  const handleSortChange = (value: string) => {
    const [by, order] = value.split('-') as [ListingSortBy, SortOrder];
    setSortBy(by);
    setSortOrder(order);
    setPage(1);
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

  return (
    <div class="max-w-6xl mx-auto px-4 py-8">
      <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 mb-6">
        <form onSubmit={handleSearchSubmit} class="flex gap-2" role="search">
          <label for="listing-search" class="sr-only">
            Search listings
          </label>
          <input
            id="listing-search"
            type="text"
            placeholder="Search listings..."
            class="border rounded px-3 py-2 text-sm"
            value={searchInput()}
            onInput={(e) => setSearchInput(e.currentTarget.value)}
          />
          <button type="submit" class="px-3 py-2 text-sm bg-blue-600 text-white rounded">
            Search
          </button>
        </form>

        <div>
          <label for="listing-sort" class="sr-only">
            Sort listings
          </label>
          <select
            id="listing-sort"
            class="border rounded px-3 py-2 text-sm"
            value={`${sortBy()}-${sortOrder()}`}
            onChange={(e) => handleSortChange(e.currentTarget.value)}
          >
            <For each={SORT_OPTIONS}>{(option) => <option value={option.value}>{option.label}</option>}</For>
          </select>
        </div>
      </div>

      <Show when={deleteError()}>
        <div class="text-red-500 text-sm bg-red-50 p-2 rounded mb-4">{deleteError()}</div>
      </Show>

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
            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-6">
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
                disabled={page() <= 1}
                onClick={() => setPage((p) => Math.max(1, p - 1))}
              >
                Previous
              </button>
              <span class="text-sm text-gray-600">
                Page {page()} of {totalPages()}
              </span>
              <button
                type="button"
                class="px-3 py-1 border rounded disabled:opacity-40"
                disabled={page() >= totalPages()}
                onClick={() => setPage((p) => Math.min(totalPages(), p + 1))}
              >
                Next
              </button>
            </div>
          </Show>
        </Show>
      </Show>
    </div>
  );
}
