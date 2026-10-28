import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, fireEvent } from '@solidjs/testing-library';
import { SearchBar } from '../SearchBar';

describe('SearchBar', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('debounces input before calling onDebouncedChange', () => {
    const onChange = vi.fn();
    render(() => <SearchBar value="" onDebouncedChange={onChange} debounceMs={200} />);

    fireEvent.input(screen.getByLabelText('Search listings'), { target: { value: '  camera ' } });
    expect(onChange).not.toHaveBeenCalled();

    vi.advanceTimersByTime(199);
    expect(onChange).not.toHaveBeenCalled();

    vi.advanceTimersByTime(1);
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith('camera');
  });

  it('only emits the latest value when typing quickly', () => {
    const onChange = vi.fn();
    render(() => <SearchBar value="" onDebouncedChange={onChange} debounceMs={200} />);

    const input = screen.getByLabelText('Search listings');
    fireEvent.input(input, { target: { value: 'cam' } });
    vi.advanceTimersByTime(100);
    fireEvent.input(input, { target: { value: 'camera' } });

    vi.advanceTimersByTime(200);
    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith('camera');
  });

  it('submits immediately and cancels the pending debounce', () => {
    const onChange = vi.fn();
    render(() => <SearchBar value="" onDebouncedChange={onChange} debounceMs={200} />);

    fireEvent.input(screen.getByLabelText('Search listings'), { target: { value: 'lamp' } });
    fireEvent.click(screen.getByText('Search'));

    expect(onChange).toHaveBeenCalledTimes(1);
    expect(onChange).toHaveBeenCalledWith('lamp');

    vi.advanceTimersByTime(500);
    expect(onChange).toHaveBeenCalledTimes(1);
  });

  it('clears the input and emits an empty query', () => {
    const onChange = vi.fn();
    render(() => <SearchBar value="camera" onDebouncedChange={onChange} debounceMs={200} />);

    fireEvent.click(screen.getByLabelText('Clear search'));

    expect((screen.getByLabelText('Search listings') as HTMLInputElement).value).toBe('');
    expect(onChange).toHaveBeenCalledWith('');
  });

  it('renders suggestions in a datalist when provided', () => {
    render(() => (
      <SearchBar
        value=""
        onDebouncedChange={() => undefined}
        suggestions={['Vintage Camera', 'electronics']}
      />
    ));

    const input = screen.getByLabelText('Search listings');
    const listId = input.getAttribute('list');
    expect(listId).toBeTruthy();

    const datalist = document.getElementById(listId as string);
    expect(datalist).toBeTruthy();
    expect(datalist?.querySelectorAll('option').length).toBe(2);
  });

  it('does not render a datalist when there are no suggestions', () => {
    render(() => <SearchBar value="" onDebouncedChange={() => undefined} suggestions={[]} />);
    expect(document.querySelector('datalist')).toBeNull();
  });
});
