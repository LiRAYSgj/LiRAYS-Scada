<script lang="ts">
	import { resolveAssetDefinition } from '$lib/features/graph/assets/registry';
	import type { PlantAssetComponentProps } from '$lib/features/graph/assets/types';

	let { data, selected = false }: PlantAssetComponentProps = $props();
	const definition = $derived(resolveAssetDefinition(data.assetKind));
</script>

<definition.component {data} {selected} />
