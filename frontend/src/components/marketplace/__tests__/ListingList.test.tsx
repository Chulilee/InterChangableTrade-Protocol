import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@solidjs/testing-library';
import { Router } from '@solidjs/router';
import { AuthProvider } from '../../../contexts/AuthContext';
import { ListingList } from '../ListingList';
import { fetchListings } from '../../../api/listings';
import type { PaginatedListings } from '../../../types/listing';

vi.mock('../../../api/listings', async () => ({
  fetchListings: vi.fn(),
  deleteListing: vi.fn()
}));

function page(overrides: Partial<PaginatedListings> = {}): PaginatedListings {
  return {
    items: [
      {
        id: '1',
        title: 'Vintage Camera',
        description: 'A well-kept vintage film camera.',
        price: 120,
        images: [],
        sellerId: 'seller-1',
        createdAt: '2026-01-15T00:00:00.000Z',
        updatedAt: '2026-01-15T00:00:00.000Z'
      }
    ],
    total: 1,
    page: 1,
    limit: 12,
    totalPages: 1,
    ...overrides
  };
}

function renderListingList() {
  return render(() => (
    <Router url="/listings">
      <AuthProvider>
        <ListingList />
      </AuthProvider>
    </Router>
  ));
}

describe('ListingList', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    global.fetch = vi.fn().mockResolvedValue({ ok: true, json: () => Promise.resolve({}) });
  });

  it('shows a loading state, then renders listing cards', async () => {
    (fetchListings as vi.Mock).mockResolvedValue(page());
    renderListingList();

    expect(screen.getByText('Loading listings...')).toBeTruthy();
    expect(await screen.findByText('Vintage Camera')).toBeTruthy();
  });

  it('shows an empty state when there are no results', async () => {
    (fetchListings as vi.Mock).mockResolvedValue(page({ items: [], total: 0, totalPages: 1 }));
    renderListingList();

    expect(await screen.findByText('No listings found.')).toBeTruthy();
  });

  it('shows an error state with a retry button on failure', async () => {
    (fetchListings as vi.Mock).mockRejectedValueOnce(new Error('Network down'));
    renderListingList();

    expect(await screen.findByText('Network down')).toBeTruthy();

    (fetchListings as vi.Mock).mockResolvedValueOnce(page());
    fireEvent.click(screen.getByText('Retry'));

    expect(await screen.findByText('Vintage Camera')).toBeTruthy();
  });

  it('disables Previous on the first page and paginates forward on Next', async () => {
    (fetchListings as vi.Mock).mockResolvedValue(page({ totalPages: 3 }));
    renderListingList();

    await screen.findByText('Vintage Camera');
    expect((screen.getByText('Previous') as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText('Page 1 of 3')).toBeTruthy();

    fireEvent.click(screen.getByText('Next'));

    await waitFor(() => {
      expect(fetchListings).toHaveBeenLastCalledWith(expect.objectContaining({ page: 2 }), expect.anything());
    });
  });

  it('resets to page 1 and passes the search term on submit', async () => {
    (fetchListings as vi.Mock).mockResolvedValue(page());
    renderListingList();
    await screen.findByText('Vintage Camera');

    fireEvent.input(screen.getByLabelText('Search listings'), { target: { value: 'camera' } });
    fireEvent.click(screen.getByText('Search'));

    await waitFor(() => {
      expect(fetchListings).toHaveBeenLastCalledWith(expect.objectContaining({ page: 1, search: 'camera' }), expect.anything());
    });
  });

  it('passes sortBy/sortOrder when the sort select changes', async () => {
    (fetchListings as vi.Mock).mockResolvedValue(page());
    renderListingList();
    await screen.findByText('Vintage Camera');

    fireEvent.change(screen.getByLabelText('Sort listings'), { target: { value: 'price-asc' } });

    await waitFor(() => {
      expect(fetchListings).toHaveBeenLastCalledWith(
        expect.objectContaining({ sortBy: 'price', sortOrder: 'asc', page: 1 }),
        expect.anything()
      );
    });
  });
});
