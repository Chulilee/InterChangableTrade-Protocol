import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@solidjs/testing-library';
import { Router } from '@solidjs/router';
import { ListingCreate } from '../ListingCreate';
import { createListing } from '../../../api/listings';

vi.mock('../../../api/listings', async () => ({
  createListing: vi.fn()
}));

vi.mock('../../../hooks/useApi', () => ({
  useApi: () => ({
    apiRequest: vi.fn(async () => new Response(JSON.stringify({}), { status: 200 })),
  }),
}));

function renderListingCreate() {
  return render(() => (
    <Router url="/listings/new">
      <ListingCreate />
    </Router>
  ));
}

describe('ListingCreate', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    sessionStorage.clear();
  });

  it('calls createListing with the validated form values on submit', async () => {
    (createListing as vi.Mock).mockResolvedValue({
      id: '1',
      title: 'Vintage Camera',
      description: 'A well-kept vintage film camera.',
      price: 120,
      images: [],
      sellerId: 'seller-1',
      createdAt: '2026-01-15T00:00:00.000Z',
      updatedAt: '2026-01-15T00:00:00.000Z'
    });

    renderListingCreate();

    fireEvent.input(screen.getByLabelText('Title'), { target: { value: 'Vintage Camera' } });
    fireEvent.input(screen.getByLabelText('Description'), {
      target: { value: 'A well-kept vintage film camera.' }
    });
    fireEvent.input(screen.getByLabelText('Price (USD)'), { target: { value: '120' } });

    fireEvent.click(screen.getByRole('button', { name: 'Create Listing' }));

    await waitFor(() => {
      expect(createListing).toHaveBeenCalledWith(
        { title: 'Vintage Camera', description: 'A well-kept vintage film camera.', price: 120, images: [] },
        expect.any(Function)
      );
    });
  });

  it('shows the API error message when creation fails', async () => {
    (createListing as vi.Mock).mockRejectedValue(new Error('Title already exists'));
    renderListingCreate();

    fireEvent.input(screen.getByLabelText('Title'), { target: { value: 'Vintage Camera' } });
    fireEvent.input(screen.getByLabelText('Description'), {
      target: { value: 'A well-kept vintage film camera.' }
    });
    fireEvent.input(screen.getByLabelText('Price (USD)'), { target: { value: '120' } });

    fireEvent.click(screen.getByRole('button', { name: 'Create Listing' }));

    expect(await screen.findByText('Title already exists')).toBeTruthy();
  });
});
