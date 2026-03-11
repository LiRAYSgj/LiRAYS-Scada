<script lang="ts">
	import BaseAssetShell from './BaseAssetShell.svelte';
	import type { PlantAssetComponentProps } from '../types';

	let { data, selected = false }: PlantAssetComponentProps = $props();
</script>

<BaseAssetShell {data} {selected} assetClass="w-[170px] h-[230px]">
	{#snippet body()}
		<svg viewBox="0 0 90 120" class="h-[130px] w-full">
			<circle cx="45" cy="58" r="24" fill="none" stroke="currentColor" stroke-width="2" />
			<circle cx="45" cy="58" r="9" fill="currentColor" />
			<rect x="12" y="53" width="16" height="10" rx="2" fill="currentColor" />
			<rect x="62" y="53" width="16" height="10" rx="2" fill="currentColor" />
			<rect x="39" y="20" width="12" height="12" rx="2" fill="currentColor" opacity="0.6" />
		</svg>
	{/snippet}
</BaseAssetShell>
