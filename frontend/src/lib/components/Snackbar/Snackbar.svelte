<script lang="ts">
	import { onDestroy } from "svelte";
	import { fade } from "svelte/transition";
	import { snackbarStore } from "$lib/stores/snackbar";

	let hideTimer: ReturnType<typeof setTimeout> | null = null;
	let renderKey = $state(0);
	let renderKeySeed = 0;

	type SnackbarTone = {
		container: string;
		progress: string;
	};

	const tone = $derived.by<SnackbarTone>(() => {
		const entry = $snackbarStore;
		if (!entry) {
			return { container: "", progress: "" };
		}
		if (entry.type === "success") {
			return {
				container: "border-emerald-600/40 bg-emerald-600/12 text-emerald-100",
				progress: "bg-emerald-500/80",
			};
		}
		if (entry.type === "warning") {
			return {
				container: "border-amber-600/40 bg-amber-600/12 text-amber-100",
				progress: "bg-amber-500/80",
			};
		}
		return {
			container: "border-destructive/45 bg-destructive/18 text-destructive",
			progress: "bg-destructive/85",
		};
	});

	const durationMs = $derived.by(() => {
		const entry = $snackbarStore;
		if (!entry) {
			return 5000;
		}
		return entry.duration ?? 5000;
	});

	$effect(() => {
		const entry = $snackbarStore;
		if (hideTimer) {
			clearTimeout(hideTimer);
			hideTimer = null;
		}
		if (!entry) {
			return;
		}
		renderKeySeed += 1;
		renderKey = renderKeySeed;
		hideTimer = setTimeout(() => {
			snackbarStore.hide();
		}, durationMs);
	});

	onDestroy(() => {
		if (hideTimer) {
			clearTimeout(hideTimer);
		}
	});
</script>

{#if $snackbarStore}
	<div class="pointer-events-none fixed right-4 top-4 z-[120]" transition:fade>
		{#key renderKey}
			<div
				class={`pointer-events-auto relative inline-flex max-w-[min(640px,calc(100vw-2rem))] items-start gap-3 overflow-hidden rounded-md border px-4 py-3 shadow-lg backdrop-blur-sm ${tone.container}`}
				role="status"
				aria-live="polite"
			>
				<div class="snackbar__timer-track absolute inset-x-0 top-0 h-[2px]">
					<div
						class={`snackbar__timer h-full ${tone.progress}`}
						style={`--snackbar-duration:${durationMs}ms;`}
					></div>
				</div>
				<p class="min-w-0 text-sm leading-relaxed break-words">{$snackbarStore.message}</p>
				<button
					type="button"
					class="shrink-0 self-start rounded border border-current/30 px-2 py-0.5 text-xs opacity-80 hover:opacity-100"
					onclick={() => snackbarStore.hide()}
				>
					Dismiss
				</button>
			</div>
		{/key}
	</div>
{/if}

<style>
	.snackbar__timer-track {
		background: color-mix(in srgb, currentColor 14%, transparent);
	}

	.snackbar__timer {
		transform-origin: left center;
		animation: snackbar-countdown var(--snackbar-duration) linear forwards;
	}

	@keyframes snackbar-countdown {
		from {
			transform: scaleX(1);
		}
		to {
			transform: scaleX(0);
		}
	}
</style>
