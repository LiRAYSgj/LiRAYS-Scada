const CONTROL_CHAR_REGEX = /[\u0000-\u001F\u007F]/g;
const MULTISPACE_REGEX = /\s+/g;

/**
 * Lightweight, client-side input normalization.
 * Security-critical protection must still happen on the backend with
 * parameterized DB queries and strict server-side validation.
 */
export function sanitizeText(input: string, maxLen = 256): string {
  return input
    .replace(CONTROL_CHAR_REGEX, "")
    .trim()
    .replace(MULTISPACE_REGEX, " ")
    .slice(0, maxLen);
}

export function sanitizeIdentifierLike(input: string, maxLen = 128): string {
  return sanitizeText(input, maxLen).replace(/[<>`"'\\;]/g, "");
}

export function sanitizeCsvList(input: string, maxLenPerItem = 64): string[] {
  return input
    .split(",")
    .map((v) => sanitizeIdentifierLike(v, maxLenPerItem))
    .filter(Boolean);
}
