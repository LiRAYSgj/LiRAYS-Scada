import { browser, dev } from "$app/environment";

/** Rust realtime server while the UI runs on the Vite dev server (different port than `location`). */
const WS_DEV_BACKEND = "ws://127.0.0.1:8245/ws";

/**
 * Single source of truth for `tagStreamClient` (tree LIST/ADD/… and realtime).
 * - **Production + browser:** same origin as the page + `/ws` (e.g. Axum).
 * - **Otherwise** (`vite dev`, or prerender / Node): `WS_DEV_BACKEND` (no socket in Node).
 */
export function resolveTagStreamWsEndpoint(): string {
  if (!dev && browser) {
    const isHttps = location.protocol === "https:";
    const scheme = isHttps ? "wss" : "ws";
    const host = location.host || location.hostname;
    return `${scheme}://${host}/ws`;
  }
  return WS_DEV_BACKEND;
}
