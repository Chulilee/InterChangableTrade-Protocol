// Client-side validation for listing create/edit forms (issue #25).

export interface ListingValidationErrors {
  title?: string;
  description?: string;
  price?: string;
  images?: string;
  category?: string;
  tags?: string;
}

export interface ListingFormValues {
  title: string;
  description: string;
  price: string | number;
  images: string[];
  category?: string;
  tags?: string[];
}

export const MAX_LISTING_IMAGES = 8;
export const MAX_LISTING_TAGS = 8;
export const MAX_CATEGORY_LENGTH = 40;
export const MAX_TAG_LENGTH = 24;
const IMAGE_URL_PATTERN = /^https?:\/\/\S+$/i;
const TAG_PATTERN = /^[a-z0-9][a-z0-9- ]*$/i;

/** Splits a comma/newline separated tag string into trimmed, non-empty tags. */
export function parseTagsInput(raw: string): string[] {
  return Array.from(
    new Set(
      raw
        .split(/[,\n]/)
        .map((entry) => entry.trim())
        .filter(Boolean)
    )
  );
}

export function validateListingInput(values: ListingFormValues): ListingValidationErrors {
  const errors: ListingValidationErrors = {};

  const title = values.title.trim();
  if (!title) {
    errors.title = 'Title is required';
  } else if (title.length < 3) {
    errors.title = 'Title must be at least 3 characters';
  } else if (title.length > 100) {
    errors.title = 'Title must be at most 100 characters';
  }

  const description = values.description.trim();
  if (!description) {
    errors.description = 'Description is required';
  } else if (description.length < 10) {
    errors.description = 'Description must be at least 10 characters';
  } else if (description.length > 2000) {
    errors.description = 'Description must be at most 2000 characters';
  }

  const rawPrice = values.price;
  const price = typeof rawPrice === 'string' ? parseFloat(rawPrice) : rawPrice;
  if (rawPrice === '' || rawPrice === null || rawPrice === undefined || Number.isNaN(price)) {
    errors.price = 'Price is required';
  } else if (price <= 0) {
    errors.price = 'Price must be greater than 0';
  }

  const images = values.images.map((url) => url.trim()).filter(Boolean);
  if (images.length > MAX_LISTING_IMAGES) {
    errors.images = `You can add up to ${MAX_LISTING_IMAGES} images`;
  } else if (images.some((url) => !IMAGE_URL_PATTERN.test(url))) {
    errors.images = 'Each image must be a valid http(s) URL';
  }

  const category = (values.category ?? '').trim();
  if (category.length > MAX_CATEGORY_LENGTH) {
    errors.category = `Category must be at most ${MAX_CATEGORY_LENGTH} characters`;
  }

  const tags = (values.tags ?? []).map((tag) => tag.trim()).filter(Boolean);
  if (tags.length > MAX_LISTING_TAGS) {
    errors.tags = `You can add up to ${MAX_LISTING_TAGS} tags`;
  } else if (tags.some((tag) => tag.length > MAX_TAG_LENGTH)) {
    errors.tags = `Each tag must be at most ${MAX_TAG_LENGTH} characters`;
  } else if (tags.some((tag) => !TAG_PATTERN.test(tag))) {
    errors.tags = 'Tags may only contain letters, numbers, spaces and dashes';
  } else if (new Set(tags.map((tag) => tag.toLowerCase())).size !== tags.length) {
    errors.tags = 'Tags must be unique';
  }

  return errors;
}

export function hasValidationErrors(errors: ListingValidationErrors): boolean {
  return Object.keys(errors).length > 0;
}
