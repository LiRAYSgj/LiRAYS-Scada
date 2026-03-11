<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte';
	import type { Snippet } from 'svelte';
	import type { PlantAssetNodeData } from '../types';

	interface Props {
		data: PlantAssetNodeData;
		selected?: boolean;
		assetClass?: string;
		body?: Snippet;
		footer?: Snippet;
	}

	let {
		data,
		selected = false,
		assetClass = 'w-[170px] h-[220px]',
		body,
		footer
	}: Props = $props();
</script>

<article
	class={`${assetClass} p-2 text-[11px] ${selected ? 'ring-1 ring-sky-500/60' : ''}`}
	style="background-color: transparent; color: var(--text-primary);"
>
	<Handle type="target" position={Position.Left} id="in" />
	<Handle type="source" position={Position.Right} id="out" />

	<header class="mb-2 flex items-center justify-between gap-2">
		<strong class="truncate text-xs">{data.title}</strong>
		<span class="px-1 py-0.5 text-[10px] tracking-wide uppercase opacity-80">
			{data.assetKind}
		</span>
	</header>

	<div class="mb-2 w-full">
		{@render body?.()}
	</div>

	<footer class="space-y-0.5 text-[10px] text-(--text-muted) opacity-85">
		<p class="truncate">Source: {data.sourceNode.name}</p>
		<p class="truncate">{data.sourceNode.path}</p>
		{@render footer?.()}
	</footer>
</article>
