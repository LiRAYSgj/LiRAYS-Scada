<script lang="ts">
	import { onDestroy } from 'svelte';
	import BaseAssetShell from './BaseAssetShell.svelte';
	import { toPercent } from '../controller';
	import type { PlantAssetComponentProps } from '../types';

	let { data, selected = false }: PlantAssetComponentProps = $props();
	let sliderValue = $derived(toPercent(data.liveValue));
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;

	onDestroy(() => {
		if (debounceTimer) {
			clearTimeout(debounceTimer);
		}
	});

	function handleInput(event: Event): void {
		const target = event.currentTarget as HTMLInputElement;
		sliderValue = Number(target.value);
		if (debounceTimer) {
			clearTimeout(debounceTimer);
		}
		debounceTimer = setTimeout(() => {
			data.onWriteValue?.(sliderValue);
		}, 300);
	}
</script>

<BaseAssetShell {data} {selected} assetClass="w-[220px] h-[170px]">
	{#snippet body()}
		<div
			class="flex h-[110px] w-full flex-col justify-center gap-2 rounded border border-black/15 p-2 dark:border-white/15"
		>
			<input
				type="range"
				min="0"
				max="100"
				step="1"
				value={sliderValue}
				oninput={handleInput}
				class="w-full cursor-pointer accent-sky-500"
			/>
			<div class="flex items-center justify-between text-[10px] text-(--text-muted)">
				<span>0%</span>
				<span class="font-semibold text-(--text-primary)">{sliderValue}%</span>
				<span>100%</span>
			</div>
		</div>
	{/snippet}
	{#snippet footer()}
		<p>Command: {sliderValue}%</p>
	{/snippet}
</BaseAssetShell>
