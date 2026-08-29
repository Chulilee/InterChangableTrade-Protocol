import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent } from '@solidjs/testing-library';
import { ListingCard } from '../ListingCard';
import type { Listing } from '../../../types/listing';

const baseListing: Listing = {
  id: '1',
  title: 'Vintage Camera',
  description: 'A well-kept vintage film camera.',
  price: 120,
  images: ['https://example.com/camera.jpg'],
  sellerId: 'seller-1',
  sellerName: 'Alice',
  createdAt: '2026-01-15T00:00:00.000Z',
  updatedAt: '2026-01-15T00:00:00.000Z'
};

describe('ListingCard', () => {
  it('renders title, description and formatted price', () => {
    const { getByText } = render(() => <ListingCard listing={baseListing} />);
    expect(getByText('Vintage Camera')).toBeTruthy();
    expect(getByText('A well-kept vintage film camera.')).toBeTruthy();
    expect(getByText('$120.00')).toBeTruthy();
  });

  it('renders the cover image when images are present', () => {
    const { getByAltText } = render(() => <ListingCard listing={baseListing} />);
    const img = getByAltText('Vintage Camera') as HTMLImageElement;
    expect(img.src).toBe('https://example.com/camera.jpg');
  });

  it('renders a placeholder when there are no images', () => {
    const { getByText } = render(() => <ListingCard listing={{ ...baseListing, images: [] }} />);
    expect(getByText('No image')).toBeTruthy();
  });

  it('hides edit/delete actions when canManage is false', () => {
    const { queryByText } = render(() => <ListingCard listing={baseListing} canManage={false} />);
    expect(queryByText('Edit')).toBeNull();
    expect(queryByText('Delete')).toBeNull();
  });

  it('invokes onEdit and onDelete when canManage is true', () => {
    const onEdit = vi.fn();
    const onDelete = vi.fn();
    const { getByText } = render(() => (
      <ListingCard listing={baseListing} canManage onEdit={onEdit} onDelete={onDelete} />
    ));

    fireEvent.click(getByText('Edit'));
    fireEvent.click(getByText('Delete'));

    expect(onEdit).toHaveBeenCalledWith(baseListing);
    expect(onDelete).toHaveBeenCalledWith(baseListing);
  });
});
