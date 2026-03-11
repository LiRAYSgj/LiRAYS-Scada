<script lang="ts">
	import BaseAssetShell from './BaseAssetShell.svelte';
	import { toBoolean } from '../controller';
	import type { PlantAssetComponentProps } from '../types';

	let { data, selected = false }: PlantAssetComponentProps = $props();
	let currentValue = $derived(toBoolean(data.liveValue));

	function toggle(): void {
		const next = !currentValue;
		currentValue = next;
		data.onWriteValue?.(next);
	}
</script>

<BaseAssetShell {data} {selected} assetClass="w-[200px] h-[170px]">
	{#snippet body()}
		<div
			class="flex h-[110px] w-full flex-col items-center justify-center gap-3 rounded border border-black/15 p-2 dark:border-white/15"
		>
			<button
				type="button"
				class={`inline-flex h-8 w-20 items-center justify-center rounded text-xs font-semibold transition ${
					currentValue ? 'bg-emerald-600 text-white' : 'bg-slate-500 text-white'
				}`}
				onclick={toggle}
			>
				{currentValue ? 'ON' : 'OFF'}
			</button>
			<span class="text-[10px] text-(--text-muted)">Writes boolean via WebSocket</span>
		</div>
	{/snippet}
	{#snippet footer()}
		<p>Command: {currentValue ? 'ON' : 'OFF'}</p>
	{/snippet}
</BaseAssetShell>
