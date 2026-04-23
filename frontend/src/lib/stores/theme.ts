import { writable } from "svelte/store";

export type ThemeMode = "light" | "dark";

const STORAGE_KEY = "app-theme";

function applyThemeClass(theme: ThemeMode): void {
  if (typeof window === "undefined") return;
  const html = window.document.documentElement;
  const active = theme === "dark" ? "theme-dark" : "theme-light";
  const inactive = theme === "dark" ? "theme-light" : "theme-dark";
  html.classList.remove(inactive);
  html.classList.add(active);
  // Keep conventional `.dark` class in sync for any third-party or legacy selectors.
  html.classList.toggle("dark", theme === "dark");
}

/** No theme is applied to the app until this is called (e.g. from layout onMount). */
export function initThemeFromStorage(): void {
  if (typeof window === "undefined") return;
  const stored = localStorage.getItem(STORAGE_KEY);
  const theme: ThemeMode =
    stored === "light" || stored === "dark" ? stored : "light";
  applyThemeClass(theme);
  themeStore.set(theme);
}

export const themeStore = writable<ThemeMode | null>(null);

themeStore.subscribe((theme) => {
  if (theme === null || typeof window === "undefined") return;

  localStorage.setItem(STORAGE_KEY, theme);
  applyThemeClass(theme);
});
