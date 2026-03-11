<script lang="ts">
	import BaseAssetShell from './BaseAssetShell.svelte';
	import type { PlantAssetComponentProps } from '../types';

	let { data, selected = false }: PlantAssetComponentProps = $props();
</script>

<BaseAssetShell {data} {selected} assetClass="w-[160px] h-[210px]">
	{#snippet body()}
		<svg viewBox="0 0 90 120" class="h-[120px] w-full">
			<rect x="8" y="55" width="74" height="10" rx="3" fill="currentColor" opacity="0.4" />
			<rect
				x="33"
				y="35"
				width="24"
				height="50"
				rx="5"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
			/>
			<line x1="45" y1="24" x2="45" y2="96" stroke="currentColor" stroke-width="2" />
		</svg>
	{/snippet}
</BaseAssetShell>
