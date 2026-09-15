import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
  buildListingsQuery,
  fetchListings,
  fetchListing,
  createListing,
  updateListing,
  deleteListing
} from '../listings';

const mockPaginated = {
  items: [
    {
      id: '1',
      title: 'Widget',
      description: 'A fine widget',
      price: 10,
      images: [],
      sellerId: 'seller-1',
      createdAt: '2026-01-01T00:00:00.000Z',
      updatedAt: '2026-01-01T00:00:00.000Z'
    }
  ],
  total: 1,
  page: 1,
  limit: 12,
  totalPages: 1
};

describe('buildListingsQuery', () => {
  it('returns an empty string for no params', () => {
    expect(buildListingsQuery()).toBe('');
    expect(buildListingsQuery({})).toBe('');
  });

  it('omits falsy/empty params', () => {
    expect(buildListingsQuery({ search: '', page: 0 })).toBe('');
  });

  it('serializes all supported params', () => {
    const qs = buildListingsQuery({ page: 2, limit: 10, search: 'lamp', sortBy: 'price', sortOrder: 'asc' });
    const params = new URLSearchParams(qs.slice(1));
    expect(params.get('page')).toBe('2');
    expect(params.get('limit')).toBe('10');
    expect(params.get('search')).toBe('lamp');
    expect(params.get('sortBy')).toBe('price');
    expect(params.get('sortOrder')).toBe('asc');
  });
});

describe('listings API client', () => {
  beforeEach(() => {
    global.fetch = vi.fn();
  });

  it('fetchListings returns parsed JSON on success', async () => {
    (fetch as vi.Mock).mockResolvedValueOnce({ ok: true, json: () => Promise.resolve(mockPaginated) });
    const result = await fetchListings({ page: 1 });
    expect(result).toEqual(mockPaginated);
    expect(fetch).toHaveBeenCalledWith(expect.stringContaining('/listings?page=1'), expect.any(Object));
  });

  it('fetchListings throws the server message on failure', async () => {
    (fetch as vi.Mock).mockResolvedValueOnce({
      ok: false,
      json: () => Promise.resolve({ message: 'Server exploded' })
    });
    await expect(fetchListings()).rejects.toThrow('Server exploded');
  });

  it('fetchListings falls back to a generic message when the body is not JSON', async () => {
    (fetch as vi.Mock).mockResolvedValueOnce({
      ok: false,
      json: () => Promise.reject(new Error('not json'))
    });
    await expect(fetchListings()).rejects.toThrow('Failed to load listings');
  });

  it('fetchListing requests the listing by id', async () => {
    (fetch as vi.Mock).mockResolvedValueOnce({ ok: true, json: () => Promise.resolve(mockPaginated.items[0]) });
    const result = await fetchListing('1');
    expect(result).toEqual(mockPaginated.items[0]);
    expect(fetch).toHaveBeenCalledWith(expect.stringContaining('/listings/1'), expect.any(Object));
  });

  it('createListing posts via the provided authed request', async () => {
    const apiRequest = vi.fn().mockResolvedValue({ ok: true, json: () => Promise.resolve(mockPaginated.items[0]) });
    const input = { title: 'Widget', description: 'A fine widget', price: 10, images: [] };
    const result = await createListing(input, apiRequest);
    expect(result).toEqual(mockPaginated.items[0]);
    expect(apiRequest).toHaveBeenCalledWith(
      expect.stringContaining('/listings'),
      expect.objectContaining({ method: 'POST', body: JSON.stringify(input) })
    );
  });

  it('createListing surfaces the error message on failure', async () => {
    const apiRequest = vi.fn().mockResolvedValue({
      ok: false,
      json: () => Promise.resolve({ message: 'Title already taken' })
    });
    await expect(createListing({ title: 'x', description: 'y', price: 1, images: [] }, apiRequest)).rejects.toThrow(
      'Title already taken'
    );
  });

  it('updateListing puts to the listing id', async () => {
    const apiRequest = vi.fn().mockResolvedValue({ ok: true, json: () => Promise.resolve(mockPaginated.items[0]) });
    await updateListing('1', { price: 15 }, apiRequest);
    expect(apiRequest).toHaveBeenCalledWith(
      expect.stringContaining('/listings/1'),
      expect.objectContaining({ method: 'PUT', body: JSON.stringify({ price: 15 }) })
    );
  });

  it('deleteListing deletes the listing id', async () => {
    const apiRequest = vi.fn().mockResolvedValue({ ok: true, json: () => Promise.resolve({}) });
    await deleteListing('1', apiRequest);
    expect(apiRequest).toHaveBeenCalledWith(
      expect.stringContaining('/listings/1'),
      expect.objectContaining({ method: 'DELETE' })
    );
  });
});
