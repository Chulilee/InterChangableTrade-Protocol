// Debounced search input with query suggestions (issue #30).
//
// The input keeps its own draft value so typing stays responsive, then calls
// `onDebouncedChange` after a short pause (or immediately on submit). Native
// `<datalist>` suggestions keep the control keyboard-accessible without a
// custom popup.

import { createEffect, createSignal, For, onCleanup, Show } from 'solid-js';

export interface SearchBarProps {
  /** Committed search value (also used to reset the input from outside). */
  value: string;
  /** Called with the trimmed value after the debounce (or on submit). */
  onDebouncedChange: (value: string) => void;
  /** Optional suggestion terms rendered in a native datalist. */
  suggestions?: string[];
  debounceMs?: number;
  placeholder?: string;
}

const MAX_SUGGESTIONS = 8;
const SUGGESTIONS_ID = 'listing-search-suggestions';

export function SearchBar(props: SearchBarProps) {
  const [input, setInput] = createSignal(props.value);
  let timer: ReturnType<typeof setTimeout> | undefined;

  const cancelPending = () => {
    if (timer !== undefined) {
      clearTimeout(timer);
      timer = undefined;
    }
  };

  const emit = (value: string) => {
    cancelPending();
    props.onDebouncedChange(value.trim());
  };

  const schedule = (value: string) => {
    cancelPending();
    timer = setTimeout(() => {
      timer = undefined;
      props.onDebouncedChange(value.trim());
    }, props.debounceMs ?? 300);
  };

  // Keep the visible input in sync when the parent resets the filters.
  createEffect(() => setInput(props.value));
  onCleanup(cancelPending);

  const suggestions = () => (props.suggestions ?? []).slice(0, MAX_SUGGESTIONS);

  return (
    <form
      role="search"
      class="flex flex-1 gap-2"
      onSubmit={(event) => {
        event.preventDefault();
        emit(input());
      }}
    >
      <label for="listing-search" class="sr-only">
        Search listings
      </label>
      <div class="relative flex-1">
        <input
          id="listing-search"
          type="search"
          autocomplete="off"
          list={suggestions().length > 0 ? SUGGESTIONS_ID : undefined}
          placeholder={props.placeholder ?? 'Search listings...'}
          class="w-full border rounded px-3 py-2 pr-8 text-sm"
          value={input()}
          onInput={(event) => {
            const value = event.currentTarget.value;
            setInput(value);
            schedule(value);
          }}
        />
        <Show when={suggestions().length > 0}>
          <datalist id={SUGGESTIONS_ID}>
            <For each={suggestions()}>{(suggestion) => <option value={suggestion} />}</For>
          </datalist>
        </Show>
        <Show when={input().length > 0}>
          <button
            type="button"
            aria-label="Clear search"
            class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-500 hover:text-gray-700"
            onClick={() => {
              setInput('');
              emit('');
            }}
          >
            &times;
          </button>
        </Show>
      </div>
      <button type="submit" class="px-3 py-2 text-sm bg-blue-600 text-white rounded">
        Search
      </button>
    </form>
  );
}
