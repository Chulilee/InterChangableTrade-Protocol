// Faceted filter panel for the listings screen (issue #30).
//
// Multi-select categories, a price range (synchronised sliders + number
// inputs) and toggleable tag chips. All controls are native form elements so
// they are keyboard and screen-reader accessible out of the box.

import { createMemo, For, Show } from 'solid-js';
import type { ListingFilters } from '../../utils/listingFilters';
import { clampPriceRange, countActiveFilters, toggleValue } from '../../utils/listingFilters';

export interface FilterPanelProps {
  filters: ListingFilters;
  availableCategories: string[];
  availableTags: string[];
  /** Upper bound for the price sliders. */
  priceCeiling: number;
  onCategoriesChange: (categories: string[]) => void;
  onTagsChange: (tags: string[]) => void;
  /** Sets both bounds at once so an inverted range can never be emitted. */
  onPriceChange: (min: number | null, max: number | null) => void;
  onReset: () => void;
}

const PRICE_HINT_ID = 'listing-price-hint';

export function FilterPanel(props: FilterPanelProps) {
  const bounds = createMemo(() =>
    clampPriceRange(props.filters.minPrice, props.filters.maxPrice, props.priceCeiling)
  );
  const activeCount = createMemo(() => countActiveFilters(props.filters));

  const handleMinSlider = (value: number) => {
    const min = Math.min(value, bounds().max);
    props.onPriceChange(min, props.filters.maxPrice);
  };

  const handleMaxSlider = (value: number) => {
    const max = Math.max(value, bounds().min);
    props.onPriceChange(props.filters.minPrice, max);
  };

  const handleNumberInput = (kind: 'min' | 'max', raw: string) => {
    const trimmed = raw.trim();
    if (trimmed === '') {
      props.onPriceChange(
        kind === 'min' ? null : props.filters.minPrice,
        kind === 'max' ? null : props.filters.maxPrice
      );
      return;
    }

    const parsed = Number(trimmed);
    if (!Number.isFinite(parsed) || parsed < 0) {
      return;
    }

    if (kind === 'min') {
      const min = props.filters.maxPrice !== null ? Math.min(parsed, props.filters.maxPrice) : parsed;
      props.onPriceChange(min, props.filters.maxPrice);
    } else {
      const max = props.filters.minPrice !== null ? Math.max(parsed, props.filters.minPrice) : parsed;
      props.onPriceChange(props.filters.minPrice, max);
    }
  };

  return (
    <aside aria-label="Listing filters" class="border rounded-lg bg-white p-4 space-y-6 h-fit">
      <div class="flex items-center justify-between gap-2">
        <h2 class="font-semibold text-gray-900">Filters</h2>
        <Show when={activeCount() > 0}>
          <span class="text-xs rounded-full bg-blue-100 text-blue-800 px-2 py-0.5" aria-hidden="true">
            {activeCount()}
          </span>
        </Show>
      </div>

      <fieldset>
        <legend class="font-medium text-sm text-gray-800 mb-2">Categories</legend>
        <Show
          when={props.availableCategories.length > 0}
          fallback={<p class="text-xs text-gray-500">No categories available yet.</p>}
        >
          <div class="space-y-1">
            <For each={props.availableCategories}>
              {(category) => (
                <label class="flex items-center gap-2 text-sm text-gray-700">
                  <input
                    type="checkbox"
                    class="rounded border-gray-300"
                    checked={props.filters.categories.includes(category)}
                    onChange={() =>
                      props.onCategoriesChange(toggleValue(props.filters.categories, category))
                    }
                  />
                  <span>{category}</span>
                </label>
              )}
            </For>
          </div>
        </Show>
      </fieldset>

      <div>
        <div class="flex items-center justify-between mb-2">
          <h3 class="font-medium text-sm text-gray-800">Price range</h3>
          <span class="text-xs text-gray-500">
            {bounds().min} &ndash; {bounds().max}
          </span>
        </div>
        <p id={PRICE_HINT_ID} class="text-xs text-gray-500 mb-2">
          Drag the sliders or enter exact amounts. Empty inputs mean no bound.
        </p>
        <label class="sr-only" for="filter-min-price">
          Minimum price
        </label>
        <input
          id="filter-min-price"
          type="range"
          min={0}
          max={props.priceCeiling}
          step={1}
          class="w-full"
          value={bounds().min}
          aria-describedby={PRICE_HINT_ID}
          aria-valuetext={`${bounds().min} or more`}
          onInput={(event) => handleMinSlider(Number(event.currentTarget.value))}
        />
        <label class="sr-only" for="filter-max-price">
          Maximum price
        </label>
        <input
          id="filter-max-price"
          type="range"
          min={0}
          max={props.priceCeiling}
          step={1}
          class="w-full"
          value={bounds().max}
          aria-describedby={PRICE_HINT_ID}
          aria-valuetext={`${bounds().max} or less`}
          onInput={(event) => handleMaxSlider(Number(event.currentTarget.value))}
        />
        <div class="flex items-center gap-2 mt-2">
          <label class="flex-1 text-xs text-gray-600">
            Min
            <input
              type="number"
              min={0}
              max={props.priceCeiling}
              class="mt-1 w-full border rounded px-2 py-1 text-sm"
              value={props.filters.minPrice ?? ''}
              onInput={(event) => handleNumberInput('min', event.currentTarget.value)}
            />
          </label>
          <label class="flex-1 text-xs text-gray-600">
            Max
            <input
              type="number"
              min={0}
              max={props.priceCeiling}
              class="mt-1 w-full border rounded px-2 py-1 text-sm"
              value={props.filters.maxPrice ?? ''}
              onInput={(event) => handleNumberInput('max', event.currentTarget.value)}
            />
          </label>
        </div>
      </div>

      <fieldset>
        <legend class="font-medium text-sm text-gray-800 mb-2">Tags</legend>
        <Show
          when={props.availableTags.length > 0}
          fallback={<p class="text-xs text-gray-500">No tags available yet.</p>}
        >
          <div class="flex flex-wrap gap-2">
            <For each={props.availableTags}>
              {(tag) => (
                <button
                  type="button"
                  aria-pressed={props.filters.tags.includes(tag)}
                  class="px-2 py-1 text-xs rounded-full border"
                  classList={{
                    'bg-blue-600 text-white border-blue-600': props.filters.tags.includes(tag),
                    'bg-white text-gray-700 border-gray-300': !props.filters.tags.includes(tag)
                  }}
                  onClick={() => props.onTagsChange(toggleValue(props.filters.tags, tag))}
                >
                  {tag}
                </button>
              )}
            </For>
          </div>
        </Show>
      </fieldset>

      <button
        type="button"
        class="w-full border rounded px-3 py-2 text-sm text-gray-700 disabled:opacity-40"
        disabled={activeCount() === 0}
        onClick={props.onReset}
      >
        Clear all filters
      </button>
    </aside>
  );
}
