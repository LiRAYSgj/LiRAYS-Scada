<script lang="ts">
	import {
		Handle,
		NodeResizer,
		Position,
		useUpdateNodeInternals,
	} from "@xyflow/svelte";
	import { resolveAssetDefinition } from '$lib/features/graph/assets/registry';
	import {
		normalizeNodePortOffsets,
		type PlantAssetComponentProps,
	} from '$lib/features/graph/assets/types';
	import CustomElementAssetHost from './CustomElementAssetHost.svelte';

	let { id, data, selected = false }: PlantAssetComponentProps = $props();
	const definition = $derived(resolveAssetDefinition(data.assetKind));
	const minWidth = $derived(definition.minWidth ?? 240);
	const minHeight = $derived(definition.minHeight ?? 160);
	const keepAspectRatio = $derived(definition.keepAspectRatio !== false);
	const portOffsets = $derived(normalizeNodePortOffsets(data.portOffsets));
	const canConnect = $derived(!data.graphReadOnly);
	const connectDraftActive = $derived(Boolean(data.connectDraftActive));
	const sourceStartConnectable = $derived(canConnect && !connectDraftActive);
	const sourceEndConnectable = false;
	const targetStartConnectable = false;
	const targetEndConnectable = $derived(canConnect);
	const canResize = $derived(
		Boolean(selected) &&
		!connectDraftActive &&
		!data.graphReadOnly &&
		definition.resizable !== false
	);
	const updateNodeInternals = useUpdateNodeInternals();

	$effect(() => {
		portOffsets.top;
		portOffsets.right;
		portOffsets.bottom;
		portOffsets.left;
		updateNodeInternals(id);
	});
</script>

<NodeResizer
	isVisible={canResize}
	minWidth={minWidth}
	minHeight={minHeight}
	keepAspectRatio={keepAspectRatio}
	handleClass="asset-node-resize-handle"
	lineClass="asset-node-resize-line"
/>

<Handle
	type="source"
	id="source-top"
	position={Position.Top}
	style={`left:${portOffsets.top}%;`}
	isConnectable={canConnect}
	isConnectableStart={sourceStartConnectable}
	isConnectableEnd={sourceEndConnectable}
/>
<Handle
	type="target"
	id="target-top"
	position={Position.Top}
	style={`left:${portOffsets.top}%;`}
	isConnectable={canConnect}
	isConnectableStart={targetStartConnectable}
	isConnectableEnd={targetEndConnectable}
/>
<Handle
	type="source"
	id="source-right"
	position={Position.Right}
	style={`top:${portOffsets.right}%;`}
	isConnectable={canConnect}
	isConnectableStart={sourceStartConnectable}
	isConnectableEnd={sourceEndConnectable}
/>
<Handle
	type="target"
	id="target-right"
	position={Position.Right}
	style={`top:${portOffsets.right}%;`}
	isConnectable={canConnect}
	isConnectableStart={targetStartConnectable}
	isConnectableEnd={targetEndConnectable}
/>
<Handle
	type="source"
	id="source-bottom"
	position={Position.Bottom}
	style={`left:${portOffsets.bottom}%;`}
	isConnectable={canConnect}
	isConnectableStart={sourceStartConnectable}
	isConnectableEnd={sourceEndConnectable}
/>
<Handle
	type="target"
	id="target-bottom"
	position={Position.Bottom}
	style={`left:${portOffsets.bottom}%;`}
	isConnectable={canConnect}
	isConnectableStart={targetStartConnectable}
	isConnectableEnd={targetEndConnectable}
/>
<Handle
	type="source"
	id="source-left"
	position={Position.Left}
	style={`top:${portOffsets.left}%;`}
	isConnectable={canConnect}
	isConnectableStart={sourceStartConnectable}
	isConnectableEnd={sourceEndConnectable}
/>
<Handle
	type="target"
	id="target-left"
	position={Position.Left}
	style={`top:${portOffsets.left}%;`}
	isConnectable={canConnect}
	isConnectableStart={targetStartConnectable}
	isConnectableEnd={targetEndConnectable}
/>

<div class="h-full w-full">
	{#if definition.runtime.kind === 'svelte'}
		<definition.runtime.component {id} {data} {selected} />
	{:else}
		<CustomElementAssetHost tagName={definition.runtime.tagName} {id} {data} {selected} />
	{/if}
</div>

<style>
	:global(.svelte-flow__handle[data-handleid^="source-"]),
	:global(.svelte-flow__handle[data-handleid^="target-"]) {
		width: 10px;
		height: 10px;
		z-index: 30;
		border-radius: 9999px;
		border: 1px solid color-mix(in srgb, var(--border) 70%, transparent);
		background: color-mix(in srgb, var(--card) 72%, #2d6cdf 28%);
	}

	:global(.svelte-flow__handle[data-handleid^="target-"]) {
		opacity: 0.85;
	}

	/* Increase resize hit area so edge drags are easier than tiny midpoint/corner targets. */
	:global(.asset-node-resize-line.left),
	:global(.asset-node-resize-line.right) {
		width: 12px;
	}

	:global(.asset-node-resize-line.top),
	:global(.asset-node-resize-line.bottom) {
		height: 12px;
	}

	:global(.asset-node-resize-handle.left),
	:global(.asset-node-resize-handle.right) {
		width: 12px;
		height: 20px;
		border-radius: 6px;
	}

	:global(.asset-node-resize-handle.top),
	:global(.asset-node-resize-handle.bottom) {
		width: 20px;
		height: 12px;
		border-radius: 6px;
	}

	:global(.asset-node-resize-handle.top.left),
	:global(.asset-node-resize-handle.top.right),
	:global(.asset-node-resize-handle.bottom.left),
	:global(.asset-node-resize-handle.bottom.right) {
		width: 12px;
		height: 12px;
	}

	:global(.svelte-flow__handle[data-handleid^="source-"]) {
		opacity: 0.95;
	}

	:global(.svelte-flow__node:not(.selected) .svelte-flow__handle[data-handleid^="source-"]) {
		opacity: 0;
		pointer-events: none;
	}

	/* Keep target handles connectable even when hidden so snapping still works. */
	:global(.svelte-flow__node:not(.selected) .svelte-flow__handle[data-handleid^="target-"]) {
		opacity: 0;
		pointer-events: auto;
	}

	/* During a connection drag, reveal target handles globally for visual guidance. */
	:global(.svelte-flow.is-connecting .svelte-flow__handle[data-handleid^="target-"]) {
		opacity: 0.95;
	}
</style>
