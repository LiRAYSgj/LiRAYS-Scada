/**
 * Client-only rendering: matches production (static SPA from Axum) and avoids SSR in `vite dev`.
 * Prerender emits a real `index.html` per route (same idea as Scully: static HTML + client bundles).
 */
export const ssr = false;
export const prerender = true;
