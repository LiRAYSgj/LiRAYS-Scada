<script lang="ts">
	import { resolveAssetDefinition } from '$lib/features/graph/assets/registry';
	import type { PlantAssetComponentProps } from '$lib/features/graph/assets/types';
	import CustomElementAssetHost from './CustomElementAssetHost.svelte';

	let { data, selected = false }: PlantAssetComponentProps = $props();
	const definition = $derived(resolveAssetDefinition(data.assetKind));
</script>

{#if definition.runtime.kind === 'svelte'}
	<definition.runtime.component {data} {selected} />
{:else}
	<CustomElementAssetHost tagName={definition.runtime.tagName} {data} {selected} />
{/if}
