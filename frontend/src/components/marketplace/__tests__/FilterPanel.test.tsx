import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@solidjs/testing-library';
import { FilterPanel } from '../FilterPanel';
import { DEFAULT_FILTERS, type ListingFilters } from '../../../utils/listingFilters';

function filters(overrides: Partial<ListingFilters> = {}): ListingFilters {
  return { ...DEFAULT_FILTERS, categories: [], tags: [], ...overrides };
}

function renderPanel(overrides: Partial<ListingFilters> = {}, handlers: Partial<{
  onCategoriesChange: (value: string[]) => void;
  onTagsChange: (value: string[]) => void;
  onPriceChange: (min: number | null, max: number | null) => void;
  onReset: () => void;
}> = {}) {
  const onCategoriesChange = handlers.onCategoriesChange ?? vi.fn();
  const onTagsChange = handlers.onTagsChange ?? vi.fn();
  const onPriceChange = handlers.onPriceChange ?? vi.fn();
  const onReset = handlers.onReset ?? vi.fn();

  render(() => (
    <FilterPanel
      filters={filters(overrides)}
      availableCategories={['Electronics', 'Photo']}
      availableTags={['rare', 'vintage']}
      priceCeiling={1000}
      onCategoriesChange={onCategoriesChange}
      onTagsChange={onTagsChange}
      onPriceChange={onPriceChange}
      onReset={onReset}
    />
  ));

  return { onCategoriesChange, onTagsChange, onPriceChange, onReset };
}

describe('FilterPanel', () => {
  it('toggles a category on and reports the full selection', () => {
    const { onCategoriesChange } = renderPanel();

    fireEvent.click(screen.getByLabelText('Electronics'));
    expect(onCategoriesChange).toHaveBeenCalledWith(['Electronics']);
  });

  it('removes an already selected category', () => {
    const { onCategoriesChange } = renderPanel({ categories: ['Electronics', 'Photo'] });

    fireEvent.click(screen.getByLabelText('Electronics'));
    expect(onCategoriesChange).toHaveBeenCalledWith(['Photo']);
  });

  it('reflects tag selection through aria-pressed', () => {
    renderPanel({ tags: ['rare'] });

    expect(screen.getByText('rare').getAttribute('aria-pressed')).toBe('true');
    expect(screen.getByText('vintage').getAttribute('aria-pressed')).toBe('false');
  });

  it('toggles a tag chip', () => {
    const { onTagsChange } = renderPanel({ tags: ['rare'] });

    fireEvent.click(screen.getByText('vintage'));
    expect(onTagsChange).toHaveBeenCalledWith(['rare', 'vintage']);

    fireEvent.click(screen.getByText('rare'));
    expect(onTagsChange).toHaveBeenCalledWith([]);
  });

  it('clamps the minimum slider so it cannot cross the maximum', () => {
    const { onPriceChange } = renderPanel({ maxPrice: 50 });

    fireEvent.input(screen.getByLabelText('Minimum price'), { target: { value: '80' } });
    expect(onPriceChange).toHaveBeenCalledWith(50, 50);
  });

  it('clamps the maximum slider so it cannot cross the minimum', () => {
    const { onPriceChange } = renderPanel({ minPrice: 200 });

    fireEvent.input(screen.getByLabelText('Maximum price'), { target: { value: '10' } });
    expect(onPriceChange).toHaveBeenCalledWith(200, 200);
  });

  it('accepts exact numbers in the text inputs', () => {
    const { onPriceChange } = renderPanel();

    fireEvent.input(screen.getByLabelText('Min'), { target: { value: '25' } });
    expect(onPriceChange).toHaveBeenCalledWith(25, null);
  });

  it('treats an empty number input as unbounded', () => {
    const { onPriceChange } = renderPanel({ minPrice: 25 });

    fireEvent.input(screen.getByLabelText('Min'), { target: { value: '' } });
    expect(onPriceChange).toHaveBeenCalledWith(null, null);
  });

  it('disables the reset button when no filters are active', () => {
    renderPanel();
    expect((screen.getByText('Clear all filters') as HTMLButtonElement).disabled).toBe(true);
  });

  it('resets filters when the button is clicked', () => {
    const { onReset } = renderPanel({ search: 'camera' });

    const button = screen.getByText('Clear all filters') as HTMLButtonElement;
    expect(button.disabled).toBe(false);

    fireEvent.click(button);
    expect(onReset).toHaveBeenCalledTimes(1);
  });

  it('shows a fallback when no facets are available yet', () => {
    render(() => (
      <FilterPanel
        filters={filters()}
        availableCategories={[]}
        availableTags={[]}
        priceCeiling={1000}
        onCategoriesChange={() => undefined}
        onTagsChange={() => undefined}
        onPriceChange={() => undefined}
        onReset={() => undefined}
      />
    ));

    expect(screen.getByText('No categories available yet.')).toBeTruthy();
    expect(screen.getByText('No tags available yet.')).toBeTruthy();
  });
});
