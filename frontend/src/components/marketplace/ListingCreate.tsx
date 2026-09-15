import { createSignal } from 'solid-js';
import { useNavigate } from '@solidjs/router';
import { ListingForm } from './ListingForm';
import { createListing } from '../../api/listings';
import { useApi } from '../../hooks/useApi';
import type { ListingInput } from '../../types/listing';

export function ListingCreate() {
  const navigate = useNavigate();
  const { apiRequest } = useApi();
  const [isSubmitting, setIsSubmitting] = createSignal(false);
  const [submitError, setSubmitError] = createSignal<string | null>(null);

  const handleSubmit = async (input: ListingInput) => {
    setIsSubmitting(true);
    setSubmitError(null);
    try {
      const listing = await createListing(input, apiRequest);
      navigate('/listings', { state: { createdListingId: listing.id } });
    } catch (error) {
      setSubmitError((error as Error).message);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <div class="max-w-2xl mx-auto px-4 py-8">
      <h2 class="text-2xl font-bold text-gray-900 mb-6">Create Listing</h2>
      <ListingForm
        submitLabel="Create Listing"
        submittingLabel="Creating..."
        isSubmitting={isSubmitting()}
        submitError={submitError()}
        onSubmit={handleSubmit}
      />
    </div>
  );
}
