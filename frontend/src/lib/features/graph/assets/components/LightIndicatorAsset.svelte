<script lang="ts">
	import BaseAssetShell from './BaseAssetShell.svelte';
	import { toBoolean } from '../controller';
	import type { PlantAssetComponentProps } from '../types';

	let { data, selected = false }: PlantAssetComponentProps = $props();
	const isOn = $derived(toBoolean(data.liveValue));
</script>

<BaseAssetShell {data} {selected} assetClass="w-[180px] h-[180px]">
	{#snippet body()}
		<div class="flex h-[110px] w-full flex-col items-center justify-center gap-2">
			<span
				class={`h-12 w-12 rounded-full border border-black/20 ${
					isOn
						? 'bg-emerald-500 shadow-[0_0_12px_rgba(16,185,129,0.7)]'
						: 'bg-red-500 shadow-[0_0_12px_rgba(239,68,68,0.7)]'
				}`}
			></span>
			<span class="text-[10px] text-(--text-muted)">{isOn ? 'ON' : 'OFF'}</span>
		</div>
	{/snippet}
	{#snippet footer()}
		<p>Status: {isOn ? 'GREEN' : 'RED'}</p>
	{/snippet}
</BaseAssetShell>
