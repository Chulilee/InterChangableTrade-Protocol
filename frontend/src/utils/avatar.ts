// Avatar upload helpers for the Profile & Settings feature (issue #28).
//
// Client-side image optimization: browsers give us a raw 3-10MP photo, but an
// avatar only renders at a couple of sizes, so a full-resolution upload is
// wasted bandwidth. `optimizeAvatarImage` downscales the image to fit within
// MAX_AVATAR_DIMENSION_PX on its longer edge and re-encodes it as WebP (with a
// JPEG fallback for environments without canvas.toBlob WebP support).

export const MAX_AVATAR_DIMENSION_PX = 512;
export const MAX_AVATAR_FILE_SIZE_BYTES = 5 * 1024 * 1024; // before optimization
export const ACCEPTED_AVATAR_TYPES = ['image/png', 'image/jpeg', 'image/webp'];

/** Validation outcome for a selected avatar file. */
export type AvatarValidation = { valid: true } | { valid: false; error: string };

export function validateAvatarFile(file: File): AvatarValidation {
  if (!ACCEPTED_AVATAR_TYPES.includes(file.type)) {
    return {
      valid: false,
      error: 'Avatar must be a PNG, JPEG or WebP image'
    };
  }
  if (file.size > MAX_AVATAR_FILE_SIZE_BYTES) {
    return {
      valid: false,
      error: 'Avatar must be smaller than 5 MB'
    };
  }
  return { valid: true };
}

/** Reads a file into a data URL for an immediate client-side preview. */
export function readAvatarPreview(file: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = () => reject(new Error('Failed to read the selected image'));
    reader.readAsDataURL(file);
  });
}

/** Data URL produced by readAvatarPreview (or any data URL). */
export function parseDataUrl(dataUrl: string): { type: string; bytes: string } | null {
  const match = /^data:([^;,]+);base64,(.*)$/.exec(dataUrl);
  if (!match) return null;
  return { type: match[1], bytes: match[2] };
}

/**
 * Pure dimension math for the downscaler: keeps the input aspect ratio and
 * never upscales; the longer edge ends up at most MAX_AVATAR_DIMENSION_PX.
 */
export function computeTargetDimensions(
  width: number,
  height: number,
  maxDimension = MAX_AVATAR_DIMENSION_PX
): { width: number; height: number } {
  if (!Number.isFinite(width) || !Number.isFinite(height) || width <= 0 || height <= 0) {
    throw new Error('Invalid image dimensions');
  }
  if (width <= maxDimension && height <= maxDimension) {
    return { width, height };
  }
  const scale = maxDimension / Math.max(width, height);
  return {
    width: Math.max(1, Math.round(width * scale)),
    height: Math.max(1, Math.round(height * scale))
  };
}

/** Returns the re-encoded MIME type, depending on WebP support. */
export function optimizeOutputType(): string {
  const canvas = document.createElement('canvas');
  canvas.width = 1;
  canvas.height = 1;
  const webpSupported = canvas.toDataURL('image/webp').startsWith('data:image/webp');
  return webpSupported ? 'image/webp' : 'image/jpeg';
}

/**
 * Downscale + re-encode an image (from any Blob/File) into an optimized
 * avatar Blob suitable for upload. Requires a DOM canvas environment (not
 * exercised in unit tests; the pure helpers above are covered instead).
 */
export async function optimizeAvatarImage(source: Blob): Promise<Blob> {
  const previewDataUrl = await readAvatarPreview(source);
  const parsed = parseDataUrl(previewDataUrl);
  if (!parsed) {
    throw new Error('Unsupported image format');
  }

  const image = new Image();
  await new Promise<void>((resolve, reject) => {
    image.onload = () => resolve();
    image.onerror = () => reject(new Error('Failed to decode the selected image'));
    image.src = previewDataUrl;
  });

  const target = computeTargetDimensions(image.naturalWidth, image.naturalHeight);
  const canvas = document.createElement('canvas');
  canvas.width = target.width;
  canvas.height = target.height;

  const ctx = canvas.getContext('2d');
  if (!ctx) {
    throw new Error('Canvas is not available in this environment');
  }
  ctx.drawImage(image, 0, 0, target.width, target.height);

  const blob = await new Promise<Blob | null>((resolve) => {
    canvas.toBlob(resolve, optimizeOutputType(), 0.85);
  });
  if (!blob) {
    throw new Error('Image optimization failed');
  }
  return blob;
}
