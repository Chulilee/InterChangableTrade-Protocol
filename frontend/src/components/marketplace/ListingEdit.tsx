import { createSignal, createResource, Show } from 'solid-js';
import { useNavigate, useParams } from '@solidjs/router';
import { ListingForm } from './ListingForm';
import { fetchListing, updateListing } from '../../api/listings';
import { useApi } from '../../hooks/useApi';
import type { ListingInput } from '../../types/listing';

export function ListingEdit() {
  const params = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { apiRequest } = useApi();
  const [listing] = createResource(() => params.id, fetchListing);
  const [isSubmitting, setIsSubmitting] = createSignal(false);
  const [submitError, setSubmitError] = createSignal<string | null>(null);

  const handleSubmit = async (input: ListingInput) => {
    setIsSubmitting(true);
    setSubmitError(null);
    try {
      const updated = await updateListing(params.id, input, apiRequest);
      navigate('/listings', { state: { updatedListingId: updated.id } });
    } catch (error) {
      setSubmitError((error as Error).message);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div class="max-w-2xl mx-auto px-4 py-8">
      <h2 class="text-2xl font-bold text-gray-900 mb-6">Edit Listing</h2>
      <Show
        when={!listing.loading}
        fallback={<p class="text-gray-500">Loading listing...</p>}
      >
        <Show
          when={!listing.error}
          fallback={
            <p class="text-red-600">{(listing.error as Error)?.message || 'Failed to load listing'}</p>
          }
        >
          <Show when={listing()}>
            {(current) => (
              <ListingForm
                initialValues={current()}
                submitLabel="Save Changes"
                submittingLabel="Saving..."
                isSubmitting={isSubmitting()}
                submitError={submitError()}
                onSubmit={handleSubmit}
              />
            )}
          </Show>
        </Show>
      </Show>
    </div>
  );
}
