import { describe, it, expect } from 'vitest';
import {
  ACCEPTED_AVATAR_TYPES,
  computeTargetDimensions,
  parseDataUrl,
  validateAvatarFile
} from '../avatar';

function makeFile(overrides: Partial<{ type: string; size: number }> = {}): File {
  return new File(['bytes'], 'avatar.png', {
    type: overrides.type ?? 'image/png',
    lastModified: Date.now()
  });
}

describe('validateAvatarFile', () => {
  it('accepts supported image types', () => {
    for (const type of ACCEPTED_AVATAR_TYPES) {
      expect(validateAvatarFile(makeFile({ type }))).toEqual({ valid: true });
    }
  });

  it('rejects unsupported types', () => {
    const result = validateAvatarFile(makeFile({ type: 'image/gif' }));
    expect(result).toEqual({
      valid: false,
      error: 'Avatar must be a PNG, JPEG or WebP image'
    });
  });

  it('rejects files above the size limit', () => {
    const result = validateAvatarFile(makeFile({ size: 5 * 1024 * 1024 + 1 }));
    expect(result).toEqual({
      valid: false,
      error: 'Avatar must be smaller than 5 MB'
    });
  });
});

describe('computeTargetDimensions', () => {
  it('keeps dimensions that already fit', () => {
    expect(computeTargetDimensions(256, 128)).toEqual({ width: 256, height: 128 });
    expect(computeTargetDimensions(512, 512)).toEqual({ width: 512, height: 512 });
  });

  it('downscales the longer edge to the cap, preserving aspect ratio', () => {
    expect(computeTargetDimensions(1024, 768)).toEqual({ width: 512, height: 384 });
    expect(computeTargetDimensions(3000, 1500)).toEqual({ width: 512, height: 256 });
    expect(computeTargetDimensions(1500, 3000)).toEqual({ width: 256, height: 512 });
  });

  it('supports a custom cap and never upscales', () => {
    expect(computeTargetDimensions(1024, 1024, 100)).toEqual({ width: 100, height: 100 });
    expect(computeTargetDimensions(50, 50, 512)).toEqual({ width: 50, height: 50 });
  });

  it('throws on invalid dimensions', () => {
    expect(() => computeTargetDimensions(0, 100)).toThrow();
    expect(() => computeTargetDimensions(-5, 100)).toThrow();
    expect(() => computeTargetDimensions(NaN, 100)).toThrow();
  });
});

describe('parseDataUrl', () => {
  it('extracts type and base64 bytes', () => {
    expect(parseDataUrl('data:image/png;base64,AAAA')).toEqual({
      type: 'image/png',
      bytes: 'AAAA'
    });
  });

  it('returns null for non-data URLs', () => {
    expect(parseDataUrl('https://example.com/avatar.png')).toBeNull();
    expect(parseDataUrl('')).toBeNull();
  });
});
