import { describe, it, expect, vi } from 'vitest';
import { render, fireEvent, screen } from '@solidjs/testing-library';
import { ListingForm } from '../ListingForm';

describe('ListingForm', () => {
  it('shows validation errors and does not submit when fields are invalid', async () => {
    const onSubmit = vi.fn();
    render(() => (
      <ListingForm
        submitLabel="Create Listing"
        submittingLabel="Creating..."
        isSubmitting={false}
        onSubmit={onSubmit}
      />
    ));

    fireEvent.click(screen.getByText('Create Listing'));

    expect(await screen.findByText('Title is required')).toBeTruthy();
    expect(screen.getByText('Description is required')).toBeTruthy();
    expect(screen.getByText('Price is required')).toBeTruthy();
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it('submits trimmed, typed values when valid', async () => {
    const onSubmit = vi.fn();
    render(() => (
      <ListingForm
        submitLabel="Create Listing"
        submittingLabel="Creating..."
        isSubmitting={false}
        onSubmit={onSubmit}
      />
    ));

    fireEvent.input(screen.getByLabelText('Title'), { target: { value: '  Vintage Camera  ' } });
    fireEvent.input(screen.getByLabelText('Description'), {
      target: { value: '  A well-kept vintage film camera.  ' }
    });
    fireEvent.input(screen.getByLabelText('Price (USD)'), { target: { value: '120' } });

    fireEvent.input(screen.getByLabelText('Images (URL)'), { target: { value: 'https://example.com/a.jpg' } });
    fireEvent.click(screen.getByText('Add'));

    fireEvent.click(screen.getByText('Create Listing'));

    expect(onSubmit).toHaveBeenCalledWith({
      title: 'Vintage Camera',
      description: 'A well-kept vintage film camera.',
      price: 120,
      images: ['https://example.com/a.jpg']
    });
  });

  it('pre-fills fields from initialValues and lets images be removed', () => {
    render(() => (
      <ListingForm
        initialValues={{
          title: 'Old Title',
          description: 'Old description here.',
          price: 50,
          images: ['https://example.com/one.jpg', 'https://example.com/two.jpg']
        }}
        submitLabel="Save Changes"
        submittingLabel="Saving..."
        isSubmitting={false}
        onSubmit={vi.fn()}
      />
    ));

    expect((screen.getByLabelText('Title') as HTMLInputElement).value).toBe('Old Title');
    expect(screen.getByText('https://example.com/one.jpg')).toBeTruthy();
    expect(screen.getByText('https://example.com/two.jpg')).toBeTruthy();

    fireEvent.click(screen.getByLabelText('Remove image 1'));
    expect(screen.queryByText('https://example.com/one.jpg')).toBeNull();
    expect(screen.getByText('https://example.com/two.jpg')).toBeTruthy();
  });

  it('disables the submit button and shows the submitting label while submitting', () => {
    render(() => (
      <ListingForm
        submitLabel="Create Listing"
        submittingLabel="Creating..."
        isSubmitting
        onSubmit={vi.fn()}
      />
    ));

    const button = screen.getByText('Creating...') as HTMLButtonElement;
    expect(button.disabled).toBe(true);
  });

  it('shows a submit error message when provided', () => {
    render(() => (
      <ListingForm
        submitLabel="Create Listing"
        submittingLabel="Creating..."
        isSubmitting={false}
        submitError="Something went wrong"
        onSubmit={vi.fn()}
      />
    ));

    expect(screen.getByText('Something went wrong')).toBeTruthy();
  });
});
