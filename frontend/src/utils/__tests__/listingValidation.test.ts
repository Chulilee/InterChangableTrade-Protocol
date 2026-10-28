import { describe, it, expect } from 'vitest';
import {
  validateListingInput,
  hasValidationErrors,
  parseTagsInput,
  MAX_LISTING_IMAGES,
  MAX_LISTING_TAGS,
  MAX_TAG_LENGTH,
  MAX_CATEGORY_LENGTH
} from '../listingValidation';

const validValues = {
  title: 'Vintage Camera',
  description: 'A well-kept vintage film camera, barely used.',
  price: 120,
  images: ['https://example.com/a.jpg']
};

describe('validateListingInput', () => {
  it('returns no errors for valid input', () => {
    const errors = validateListingInput(validValues);
    expect(hasValidationErrors(errors)).toBe(false);
  });

  it('requires a title', () => {
    const errors = validateListingInput({ ...validValues, title: '' });
    expect(errors.title).toBe('Title is required');
  });

  it('rejects a title that is too short', () => {
    const errors = validateListingInput({ ...validValues, title: 'ab' });
    expect(errors.title).toMatch(/at least 3 characters/);
  });

  it('rejects a title that is too long', () => {
    const errors = validateListingInput({ ...validValues, title: 'a'.repeat(101) });
    expect(errors.title).toMatch(/at most 100 characters/);
  });

  it('requires a description', () => {
    const errors = validateListingInput({ ...validValues, description: '   ' });
    expect(errors.description).toBe('Description is required');
  });

  it('rejects a description that is too short', () => {
    const errors = validateListingInput({ ...validValues, description: 'too short' });
    expect(errors.description).toMatch(/at least 10 characters/);
  });

  it('requires a price', () => {
    const errors = validateListingInput({ ...validValues, price: '' });
    expect(errors.price).toBe('Price is required');
  });

  it('rejects a non-numeric price', () => {
    const errors = validateListingInput({ ...validValues, price: 'abc' });
    expect(errors.price).toBe('Price is required');
  });

  it('rejects a zero or negative price', () => {
    expect(validateListingInput({ ...validValues, price: 0 }).price).toMatch(/greater than 0/);
    expect(validateListingInput({ ...validValues, price: -5 }).price).toMatch(/greater than 0/);
  });

  it('accepts a valid string price', () => {
    const errors = validateListingInput({ ...validValues, price: '19.99' });
    expect(errors.price).toBeUndefined();
  });

  it('rejects non-URL image entries', () => {
    const errors = validateListingInput({ ...validValues, images: ['not-a-url'] });
    expect(errors.images).toMatch(/valid http/);
  });

  it('accepts empty images (images are optional)', () => {
    const errors = validateListingInput({ ...validValues, images: [] });
    expect(errors.images).toBeUndefined();
  });

  it('rejects more than the maximum number of images', () => {
    const images = Array.from({ length: MAX_LISTING_IMAGES + 1 }, (_, i) => `https://example.com/${i}.jpg`);
    const errors = validateListingInput({ ...validValues, images });
    expect(errors.images).toMatch(new RegExp(`up to ${MAX_LISTING_IMAGES} images`));
  });
});

describe('validateListingInput - category and tags', () => {
  it('treats category and tags as optional', () => {
    const errors = validateListingInput(validValues);
    expect(errors.category).toBeUndefined();
    expect(errors.tags).toBeUndefined();
  });

  it('accepts a normal category and tag set', () => {
    const errors = validateListingInput({
      ...validValues,
      category: 'Electronics',
      tags: ['vintage', 'camera']
    });
    expect(hasValidationErrors(errors)).toBe(false);
  });

  it('rejects an over-long category', () => {
    const errors = validateListingInput({ ...validValues, category: 'a'.repeat(MAX_CATEGORY_LENGTH + 1) });
    expect(errors.category).toMatch(new RegExp(`at most ${MAX_CATEGORY_LENGTH} characters`));
  });

  it('rejects too many tags', () => {
    const tags = Array.from({ length: MAX_LISTING_TAGS + 1 }, (_, i) => `tag${i}`);
    const errors = validateListingInput({ ...validValues, tags });
    expect(errors.tags).toMatch(new RegExp(`up to ${MAX_LISTING_TAGS} tags`));
  });

  it('rejects an over-long tag', () => {
    const errors = validateListingInput({ ...validValues, tags: ['a'.repeat(MAX_TAG_LENGTH + 1)] });
    expect(errors.tags).toMatch(new RegExp(`at most ${MAX_TAG_LENGTH} characters`));
  });

  it('rejects tags with unsupported characters', () => {
    const errors = validateListingInput({ ...validValues, tags: ['ok', 'bad!tag'] });
    expect(errors.tags).toMatch(/letters, numbers, spaces and dashes/);
  });

  it('rejects duplicate tags regardless of case', () => {
    const errors = validateListingInput({ ...validValues, tags: ['Camera', 'camera'] });
    expect(errors.tags).toBe('Tags must be unique');
  });
});

describe('parseTagsInput', () => {
  it('splits on commas and newlines, trimming and dropping empties', () => {
    expect(parseTagsInput('a, b\n c ,,')).toEqual(['a', 'b', 'c']);
  });

  it('de-duplicates entries', () => {
    expect(parseTagsInput('vintage, Vintage, camera')).toEqual(['vintage', 'camera']);
  });

  it('returns an empty list for blank input', () => {
    expect(parseTagsInput('')).toEqual([]);
    expect(parseTagsInput(' , ')).toEqual([]);
  });
});
