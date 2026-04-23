import { browser } from "$app/environment";

/** Fallback realtime server endpoint when no browser location is available (SSR/tests). */
const WS_DEV_BACKEND = "ws://127.0.0.1:8245/ws";

/**
 * Single source of truth for `tagStreamClient` (tree LIST/ADD/… and realtime).
 * - **Browser (dev + prod):** same origin as the page + `/ws`.
 *   In dev this allows Vite WS proxy middleware to forward upgrades to backend.
 * - **Otherwise** (prerender / Node): fallback endpoint string.
 */
export function resolveTagStreamWsEndpoint(): string {
  if (browser) {
    const isHttps = location.protocol === "https:";
    const scheme = isHttps ? "wss" : "ws";
    const host = location.host || location.hostname;
    return `${scheme}://${host}/ws`;
  }
  return WS_DEV_BACKEND;
}
