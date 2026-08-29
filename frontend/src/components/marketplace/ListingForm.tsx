import { createSignal, createEffect, For, Show } from 'solid-js';
import {
  validateListingInput,
  hasValidationErrors,
  type ListingValidationErrors
} from '../../utils/listingValidation';
import type { ListingInput } from '../../types/listing';

export interface ListingFormProps {
  initialValues?: Partial<ListingInput>;
  submitLabel: string;
  submittingLabel: string;
  isSubmitting: boolean;
  submitError?: string | null;
  onSubmit: (input: ListingInput) => void | Promise<void>;
}

/**
 * Shared field set + client-side validation for creating and editing a
 * listing. `ListingCreate` and `ListingEdit` both render this and only
 * differ in how they load initial data and where they send the result.
 */
export function ListingForm(props: ListingFormProps) {
  const [title, setTitle] = createSignal(props.initialValues?.title ?? '');
  const [description, setDescription] = createSignal(props.initialValues?.description ?? '');
  const [price, setPrice] = createSignal(
    props.initialValues?.price !== undefined ? String(props.initialValues.price) : ''
  );
  const [images, setImages] = createSignal<string[]>(props.initialValues?.images ?? []);
  const [imageInput, setImageInput] = createSignal('');
  const [errors, setErrors] = createSignal<ListingValidationErrors>({});

  // Edit forms load their initial values asynchronously (after fetching the
  // listing), so keep the fields in sync if they change after first render.
  createEffect(() => {
    const initial = props.initialValues;
    if (!initial) return;
    setTitle(initial.title ?? '');
    setDescription(initial.description ?? '');
    setPrice(initial.price !== undefined ? String(initial.price) : '');
    setImages(initial.images ?? []);
  });

  const addImage = () => {
    const url = imageInput().trim();
    if (!url) return;
    setImages((prev) => [...prev, url]);
    setImageInput('');
  };

  const removeImage = (index: number) => {
    setImages((prev) => prev.filter((_, i) => i !== index));
  };

  const handleSubmit = async (e: SubmitEvent) => {
    e.preventDefault();
    const values = { title: title(), description: description(), price: price(), images: images() };
    const validationErrors = validateListingInput(values);
    setErrors(validationErrors);
    if (hasValidationErrors(validationErrors)) return;

    await props.onSubmit({
      title: title().trim(),
      description: description().trim(),
      price: parseFloat(price()),
      images: images().map((url) => url.trim()).filter(Boolean)
    });
  };

  return (
    <form onSubmit={handleSubmit} class="space-y-4 max-w-xl" novalidate>
      <div>
        <label for="listing-title" class="block text-sm font-medium text-gray-700">
          Title
        </label>
        <input
          id="listing-title"
          type="text"
          class="mt-1 block w-full border rounded px-3 py-2"
          value={title()}
          onInput={(e) => setTitle(e.currentTarget.value)}
        />
        <Show when={errors().title}>
          <p class="text-red-500 text-xs mt-1">{errors().title}</p>
        </Show>
      </div>

      <div>
        <label for="listing-description" class="block text-sm font-medium text-gray-700">
          Description
        </label>
        <textarea
          id="listing-description"
          class="mt-1 block w-full border rounded px-3 py-2"
          rows="4"
          value={description()}
          onInput={(e) => setDescription(e.currentTarget.value)}
        />
        <Show when={errors().description}>
          <p class="text-red-500 text-xs mt-1">{errors().description}</p>
        </Show>
      </div>

      <div>
        <label for="listing-price" class="block text-sm font-medium text-gray-700">
          Price (USD)
        </label>
        <input
          id="listing-price"
          type="number"
          min="0"
          step="0.01"
          class="mt-1 block w-full border rounded px-3 py-2"
          value={price()}
          onInput={(e) => setPrice(e.currentTarget.value)}
        />
        <Show when={errors().price}>
          <p class="text-red-500 text-xs mt-1">{errors().price}</p>
        </Show>
      </div>

      <div>
        <label for="listing-image-url" class="block text-sm font-medium text-gray-700">
          Images (URL)
        </label>
        <div class="flex gap-2 mt-1">
          <input
            id="listing-image-url"
            type="text"
            placeholder="https://example.com/image.jpg"
            class="flex-1 border rounded px-3 py-2"
            value={imageInput()}
            onInput={(e) => setImageInput(e.currentTarget.value)}
          />
          <button type="button" class="px-3 py-2 border rounded" onClick={addImage}>
            Add
          </button>
        </div>
        <Show when={errors().images}>
          <p class="text-red-500 text-xs mt-1">{errors().images}</p>
        </Show>
        <ul class="mt-2 space-y-1">
          <For each={images()}>
            {(url, index) => (
              <li class="flex items-center justify-between text-sm bg-gray-50 px-2 py-1 rounded">
                <span class="truncate">{url}</span>
                <button
                  type="button"
                  class="text-red-500 ml-2"
                  onClick={() => removeImage(index())}
                  aria-label={`Remove image ${index() + 1}`}
                >
                  Remove
                </button>
              </li>
            )}
          </For>
        </ul>
      </div>

      <Show when={props.submitError}>
        <div class="text-red-500 text-sm bg-red-50 p-2 rounded">{props.submitError}</div>
      </Show>

      <button
        type="submit"
        disabled={props.isSubmitting}
        class="w-full bg-blue-600 text-white py-2 rounded disabled:bg-blue-300"
      >
        {props.isSubmitting ? props.submittingLabel : props.submitLabel}
      </button>
    </form>
  );
}
