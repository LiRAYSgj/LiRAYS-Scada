<script lang="ts">
	import { afterNavigate, beforeNavigate, goto, preloadCode } from "$app/navigation";
	import { navigating } from "$app/stores";
	import { onMount } from "svelte";
	import "./layout.css";
	import Snackbar from "$lib/components/Snackbar/Snackbar.svelte";
	import { snackbarStore } from "$lib/stores/snackbar";
	import { initThemeFromStorage } from "$lib/stores/theme";

	const NAV_REVEAL_DELAY_MS = 500;
	const NAV_FINISH_HIDE_MS = 180;
	const NAV_FAILURE_DEBOUNCE_MS = 1000;

	let { children } = $props();
	let navProgress = $state(0);
	let navVisible = $state(false);
	let navigationPending = $state(false);
	let navToken = $state(0);
	let wasNavigating = $state(false);
	let guardPreloading = $state(false);
	let progressTimer = $state<ReturnType<typeof setInterval> | null>(null);
	let showDelayTimer = $state<ReturnType<typeof setTimeout> | null>(null);
	let finishTimer = $state<ReturnType<typeof setTimeout> | null>(null);
	let lastNavFailureAt = $state(0);
	let bypassGuardTarget = $state<string | null>(null);
	let guardTarget = $state<string | null>(null);
	let guardAttemptId = $state(0);
	let lastFailedChunkTarget = $state<string | null>(null);

	beforeNavigate((navigation) => {
		if (!navigation.to || navigation.willUnload) {
			return;
		}
		const targetHref = navigation.to.url.href;
		if (bypassGuardTarget === targetHref) {
			bypassGuardTarget = null;
			return;
		}
		if (lastFailedChunkTarget === targetHref) {
			lastFailedChunkTarget = null;
			navigation.cancel();
			resetNavigationProgressImmediately();
			window.location.assign(targetHref);
			return;
		}
		navigation.cancel();
		const attemptId = ++guardAttemptId;
		guardTarget = targetHref;
		guardPreloading = true;
		const targetPath = navigation.to.url.pathname;
		const targetSearch = navigation.to.url.search;
		const targetHash = navigation.to.url.hash;
		const shouldReplaceState = navigation.type === "popstate";

		void (async () => {
			try {
				await preloadCode(targetPath);
				if (attemptId !== guardAttemptId) {
					return;
				}
				bypassGuardTarget = targetHref;
				await goto(`${targetPath}${targetSearch}${targetHash}`, {
					replaceState: shouldReplaceState,
				});
			} catch (error) {
				if (attemptId !== guardAttemptId) {
					return;
				}
				const message = errorToMessage(error).toLowerCase();
				if (
					message.includes("navigation cancelled") ||
					message.includes("navigation canceled")
				) {
					return;
				}
				notifyNavigationFailure("Navigation failed. Could not load page resources.");
			} finally {
				if (attemptId === guardAttemptId) {
					guardTarget = null;
					guardPreloading = false;
				}
			}
		})();
	});

	afterNavigate((navigation) => {
		const current = navigation.to?.url.href ?? window.location.href;
		if (guardTarget === current) {
			guardTarget = null;
		}
	});

	function clearProgressTimer(): void {
		if (progressTimer) {
			clearInterval(progressTimer);
			progressTimer = null;
		}
	}

	function clearFinishTimer(): void {
		if (finishTimer) {
			clearTimeout(finishTimer);
			finishTimer = null;
		}
	}

	function clearShowDelayTimer(): void {
		if (showDelayTimer) {
			clearTimeout(showDelayTimer);
			showDelayTimer = null;
		}
	}

	function resetNavigationProgressImmediately(): void {
		navigationPending = false;
		navToken += 1;
		clearShowDelayTimer();
		clearProgressTimer();
		clearFinishTimer();
		navVisible = false;
		navProgress = 0;
	}

	function startProgressTicker(): void {
		if (progressTimer) {
			return;
		}
		progressTimer = setInterval(() => {
			if (navProgress >= 90) {
				return;
			}
			if (navProgress < 72) {
				navProgress = Math.min(90, navProgress + (72 - navProgress) * 0.26 + 1.6);
				return;
			}
			navProgress = Math.min(90, navProgress + (90 - navProgress) * 0.09 + 0.18);
		}, 120);
	}

	function revealNavigationProgress(): void {
		clearFinishTimer();
		if (!navVisible) {
			navVisible = true;
			navProgress = 5;
		} else {
			navProgress = Math.min(navProgress, 88);
		}
		startProgressTicker();
	}

	function beginNavigationProgress(): void {
		navigationPending = true;
		navToken += 1;
		const token = navToken;
		clearFinishTimer();
		if (navVisible) {
			revealNavigationProgress();
			return;
		}
		if (showDelayTimer) {
			return;
		}
		showDelayTimer = setTimeout(() => {
			showDelayTimer = null;
			if (!navigationPending || token !== navToken) {
				return;
			}
			revealNavigationProgress();
		}, NAV_REVEAL_DELAY_MS);
	}

	function completeNavigationProgress(): void {
		navigationPending = false;
		const token = navToken;
		clearShowDelayTimer();
		clearProgressTimer();
		if (!navVisible) {
			navProgress = 0;
			return;
		}
		navProgress = 100;
		clearFinishTimer();
		finishTimer = setTimeout(() => {
			if (navigationPending || token !== navToken) {
				finishTimer = null;
				return;
			}
			navVisible = false;
			navProgress = 0;
			finishTimer = null;
		}, NAV_FINISH_HIDE_MS);
	}

	function errorToMessage(value: unknown): string {
		if (typeof value === "string") {
			return value;
		}
		if (value instanceof Error) {
			return value.message || value.name;
		}
		if (
			value &&
			typeof value === "object" &&
			"message" in value &&
			typeof (value as { message?: unknown }).message === "string"
		) {
			return (value as { message: string }).message;
		}
		return "";
	}

	function isChunkLoadFailure(input: unknown): boolean {
		const message = errorToMessage(input).toLowerCase();
		if (
			message.includes("failed to fetch dynamically imported module") ||
			message.includes("importing a module script failed") ||
			message.includes("chunkloaderror") ||
			message.includes("loading chunk") ||
			message.includes("/_app/immutable/")
		) {
			return true;
		}
		if (
			input &&
			typeof input === "object" &&
			"filename" in input &&
			typeof (input as { filename?: unknown }).filename === "string"
		) {
			const filename = (input as { filename: string }).filename;
			if (filename.includes("/_app/immutable/")) {
				return true;
			}
		}
		return false;
	}

	function notifyNavigationFailure(message = "Could not load page resources."): void {
		if (guardTarget) {
			lastFailedChunkTarget = guardTarget;
		}
		resetNavigationProgressImmediately();
		bypassGuardTarget = null;
		guardTarget = null;
		guardPreloading = false;
		guardAttemptId += 1;
		const now = Date.now();
		if (now - lastNavFailureAt < NAV_FAILURE_DEBOUNCE_MS) {
			return;
		}
		lastNavFailureAt = now;
		snackbarStore.error(message);
	}

	$effect(() => {
		const isNavigating = Boolean($navigating) || guardPreloading;
		if (isNavigating === wasNavigating) {
			return;
		}
		wasNavigating = isNavigating;
		if (isNavigating) {
			beginNavigationProgress();
			return;
		}
		completeNavigationProgress();
	});

	onMount(() => {
		initThemeFromStorage();

		const handleWindowError = (event: ErrorEvent): void => {
			if (!isChunkLoadFailure(event.error ?? event.message ?? event)) {
				return;
			}
			notifyNavigationFailure();
		};

		const handleUnhandledRejection = (event: PromiseRejectionEvent): void => {
			if (!isChunkLoadFailure(event.reason)) {
				return;
			}
			notifyNavigationFailure();
		};

		window.addEventListener("error", handleWindowError);
		window.addEventListener("unhandledrejection", handleUnhandledRejection);

		return () => {
			window.removeEventListener("error", handleWindowError);
			window.removeEventListener("unhandledrejection", handleUnhandledRejection);
			clearProgressTimer();
			clearShowDelayTimer();
			clearFinishTimer();
		};
	});
</script>

<div class="app-nav-progress" aria-hidden={!navVisible} data-visible={navVisible}>
	<div
		class="app-nav-progress__bar"
		style={`transform: scaleX(${Math.max(0, Math.min(navProgress, 100)) / 100})`}
	></div>
</div>
{@render children()}
<Snackbar />

<style>
	.app-nav-progress {
		position: fixed;
		top: 0;
		left: 0;
		right: 0;
		z-index: 80;
		height: 3px;
		pointer-events: none;
		opacity: 0;
		transition: opacity 120ms ease-out;
	}

	.app-nav-progress[data-visible="true"] {
		opacity: 1;
	}

	.app-nav-progress__bar {
		height: 100%;
		width: 100%;
		transform-origin: left center;
		background: linear-gradient(
			90deg,
			color-mix(in srgb, var(--primary) 82%, white 18%) 0%,
			var(--primary) 58%,
			color-mix(in srgb, var(--primary) 86%, white 14%) 100%
		);
		box-shadow: 0 0 10px color-mix(in srgb, var(--primary) 45%, transparent);
		transition: transform 140ms ease-out;
	}
</style>
