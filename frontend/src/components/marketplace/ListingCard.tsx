import { Show } from 'solid-js';
import type { Listing } from '../../types/listing';

export interface ListingCardProps {
  listing: Listing;
  /** Show edit/delete actions - typically gated on the viewer owning the listing. */
  canManage?: boolean;
  onEdit?: (listing: Listing) => void;
  onDelete?: (listing: Listing) => void;
}

const currencyFormatter = new Intl.NumberFormat('en-US', {
  style: 'currency',
  currency: 'USD'
});

export function ListingCard(props: ListingCardProps) {
  const coverImage = () => props.listing.images[0];
  const formattedPrice = () => currencyFormatter.format(props.listing.price);
  const formattedDate = () => new Date(props.listing.createdAt).toLocaleDateString();

  return (
    <div class="border rounded-lg shadow-sm overflow-hidden bg-white flex flex-col" data-testid="listing-card">
      <Show
        when={coverImage()}
        fallback={
          <div class="h-40 bg-gray-100 flex items-center justify-center text-gray-400 text-sm">
            No image
          </div>
        }
      >
        <img src={coverImage()} alt={props.listing.title} class="h-40 w-full object-cover" />
      </Show>
      <div class="p-4 flex-1 flex flex-col">
        <h3 class="text-lg font-semibold text-gray-900 truncate">{props.listing.title}</h3>
        <p class="text-sm text-gray-600 mt-1 line-clamp-2">{props.listing.description}</p>
        <div class="mt-3 flex items-center justify-between">
          <span class="text-blue-600 font-bold">{formattedPrice()}</span>
          <span class="text-xs text-gray-400">{formattedDate()}</span>
        </div>
        <Show when={props.canManage}>
          <div class="mt-3 flex gap-3">
            <button
              type="button"
              class="text-sm text-blue-600 hover:underline"
              onClick={() => props.onEdit?.(props.listing)}
            >
              Edit
            </button>
            <button
              type="button"
              class="text-sm text-red-600 hover:underline"
              onClick={() => props.onDelete?.(props.listing)}
            >
              Delete
            </button>
          </div>
        </Show>
      </div>
    </div>
  );
}
