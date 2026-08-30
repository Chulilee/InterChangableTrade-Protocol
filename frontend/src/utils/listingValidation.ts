// Client-side validation for listing create/edit forms (issue #25).

export interface ListingValidationErrors {
  title?: string;
  description?: string;
  price?: string;
  images?: string;
}

export interface ListingFormValues {
  title: string;
  description: string;
  price: string | number;
  images: string[];
}

export const MAX_LISTING_IMAGES = 8;
const IMAGE_URL_PATTERN = /^https?:\/\/\S+$/i;

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

  return errors;
}

export function hasValidationErrors(errors: ListingValidationErrors): boolean {
  return Object.keys(errors).length > 0;
}
