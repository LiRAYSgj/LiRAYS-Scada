<script lang="ts">
	import BaseAssetShell from './BaseAssetShell.svelte';
	import { toText } from '../controller';
	import type { PlantAssetComponentProps } from '../types';

	let { data, selected = false }: PlantAssetComponentProps = $props();
	const textValue = $derived(toText(data.liveValue));
</script>

<BaseAssetShell {data} {selected} assetClass="w-[240px] h-[150px]">
	{#snippet body()}
		<div
			class="flex h-[110px] w-full items-center justify-center rounded border border-black/15 px-2 py-1 text-center text-lg font-semibold dark:border-white/15"
		>
			<span class="truncate">{textValue}</span>
		</div>
	{/snippet}
	{#snippet footer()}
		<p>Text: {textValue}</p>
	{/snippet}
</BaseAssetShell>
