import { writable } from "svelte/store";

export type ThemeMode = "light" | "dark";

const STORAGE_KEY = "app-theme";

/** No theme is applied to the app until this is called (e.g. from layout onMount). */
export function initThemeFromStorage(): void {
	if (typeof window === "undefined") return;
	const stored = localStorage.getItem(STORAGE_KEY);
	const theme: ThemeMode = stored === "light" || stored === "dark" ? stored : "light";
	themeStore.set(theme);
}

export const themeStore = writable<ThemeMode | null>(null);

themeStore.subscribe((theme) => {
	if (theme !== null && typeof window !== "undefined") {
		localStorage.setItem(STORAGE_KEY, theme);
	}
});
