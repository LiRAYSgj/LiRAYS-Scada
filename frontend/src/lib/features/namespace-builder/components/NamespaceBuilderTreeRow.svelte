<script lang="ts">
	import { Button } from "$lib/components/Button";
	import type { FlatRow } from "../types.js";

	let {
		row,
		allowedDataTypes,
		rowHeight,
		actionsDisabled = false,
		draggedNodeId,
		dropTarget,
		editingNodeId,
		editingName = $bindable(""),
		editingInputEl = $bindable<HTMLInputElement | null>(null),
		onPointerDownDrag,
		onRemove,
		onAddChild,
		onIndent,
		onOutdent,
		onStartEditName,
		onCommitEditName,
		onUpdateRange,
		onUpdateType
	}: {
		row: FlatRow;
		allowedDataTypes: string[];
		rowHeight: number;
		actionsDisabled?: boolean;
		draggedNodeId: string | null;
		dropTarget: {
			rowId: string;
			position: "before" | "child";
			allowed: boolean;
		} | null;
		editingNodeId: string | null;
		editingName?: string;
		editingInputEl?: HTMLInputElement | null;
		onPointerDownDrag: (e: PointerEvent, rowId: string) => void;
		onRemove: (rowId: string) => void;
		onAddChild: (rowId: string) => void;
		onIndent: (rowId: string) => void;
		onOutdent: (rowId: string) => void;
		onStartEditName: (rowId: string) => void;
		onCommitEditName: (rowId: string) => void;
		onUpdateRange: (
			rowId: string,
			key: "rangeStart" | "rangeEnd" | "rangeStep",
			value: string
		) => void;
		onUpdateType: (rowId: string, dataType: string) => void;
	} = $props();
</script>

<div
	class={`group/row box-border flex min-h-9 items-center border-b transition-opacity ${draggedNodeId === row.id ? "opacity-[0.35]" : ""} ${
		dropTarget?.rowId === row.id
			? dropTarget.allowed
				? dropTarget.position === "before"
					? "ns-drop-before"
					: "ns-drop-child"
				: "ns-drop-forbidden"
			: ""
	}`}
	style="min-height: {rowHeight}px; border-color: color-mix(in srgb, var(--text-muted) 18%, transparent)"
	role="listitem"
	data-node-row-id={row.id}
>
	<div
		class="inline-flex items-center gap-0.5 px-1.5 opacity-0 transition-opacity duration-100 group-hover/row:opacity-100"
	>
		<Button
			variant="icon"
			title="Drag node"
			ariaLabel="Drag node"
			disabled={actionsDisabled}
			onpointerdown={(e) => onPointerDownDrag(e, row.id)}
			class="ns-row-btn ns-drag-handle cursor-grab active:cursor-grabbing"
		>
			{#snippet children()}
				<span
					class="grid grid-cols-2 gap-0.5 text-current"
					aria-hidden="true"
					style="grid-template-columns: repeat(2, 3px); grid-template-rows: repeat(3, 3px)"
				>
					{#each Array(6) as _}
						<span class="h-[3px] w-[3px] rounded-full bg-current opacity-85"></span>
					{/each}
				</span>
			{/snippet}
		</Button>
		<Button
			variant="icon"
			label="-"
			title="Remove node"
			disabled={actionsDisabled}
			onclick={() => onRemove(row.id)}
			class="ns-row-btn"
		/>
		<Button
			variant="icon"
			label="+"
			title="Add child node"
			disabled={actionsDisabled}
			onclick={() => onAddChild(row.id)}
			class="ns-row-btn"
		/>
		<Button
			variant="icon"
			label="›"
			title="Indent (make child of above)"
			disabled={actionsDisabled || row.index === 0}
			onclick={() => onIndent(row.id)}
			class="ns-row-btn"
		/>
		<Button
			variant="icon"
			label="‹"
			title="Outdent (to parent sibling)"
			disabled={actionsDisabled || !row.parentId}
			onclick={() => onOutdent(row.id)}
			class="ns-row-btn"
		/>
	</div>
	<div
		class="ns-node-content relative flex w-full items-center gap-2.5 py-1 pr-2 pl-0"
		style={`padding-left:${row.depth * 24}px`}
	>
		{#if row.depth > 0}
			<div
				class="tree-indent-guides"
				style={`width:${row.depth * 18}px`}
				aria-hidden="true"
			>
				{#each Array(row.depth) as _, i}
					<span
						class="tree-indent-guide-vertical"
						style={`left:${i * 18 + 17}px`}
					></span>
				{/each}
				<svg
					class="tree-indent-guide-branch"
					style={`left:${(row.depth - 1) * 18}px;width:${18}px`}
					viewBox="0 0 18 12"
					preserveAspectRatio="none"
					aria-hidden="true"
				>
					<path
						class="tree-indent-guide-branch-path"
						d="M 0 1.8 C 2 4.2 4.5 6 8 6 L 18 6"
						fill="none"
						vector-effect="non-scaling-stroke"
					/>
				</svg>
			</div>
		{/if}
		{#if editingNodeId === row.id}
			<input
				id={`ns-name-${row.id}`}
				name={`ns-name-${row.id}`}
				bind:this={editingInputEl}
				class="min-w-[180px] max-w-[320px] rounded-md border border-blue-600 bg-(--bg-muted) px-1.5 py-0.5 text-[13px] text-(--text-primary) disabled:cursor-not-allowed disabled:opacity-50"
				bind:value={editingName}
				disabled={actionsDisabled}
				onkeydown={(event) => {
					if (event.key === "Enter") {
						event.preventDefault();
						onCommitEditName(row.id);
					}
				}}
				onblur={() => onCommitEditName(row.id)}
			/>
		{:else}
			<Button
				variant="ghost"
				label={row.node.name}
				disabled={actionsDisabled}
				ondblclick={() => !actionsDisabled && onStartEditName(row.id)}
				class="min-w-[150px] justify-start py-0 text-left text-[13px] cursor-text"
			/>
		{/if}
		<div class="flex flex-wrap items-center gap-1.5">
			<input
				id={`ns-${row.id}-rangeStart`}
				name={`ns-${row.id}-rangeStart`}
				class="w-[62px] rounded-md border bg-(--bg-muted) px-1 py-0.5 text-[11px] text-(--text-primary) disabled:cursor-not-allowed disabled:opacity-50"
				style="border-color: color-mix(in srgb, var(--text-muted) 28%, transparent)"
				placeholder="start"
				value={row.node.rangeStart}
				disabled={actionsDisabled}
				oninput={(event) =>
					onUpdateRange(row.id, "rangeStart", (event.currentTarget as HTMLInputElement).value)}
			/>
			<input
				id={`ns-${row.id}-rangeEnd`}
				name={`ns-${row.id}-rangeEnd`}
				class="w-[62px] rounded-md border bg-(--bg-muted) px-1 py-0.5 text-[11px] text-(--text-primary) disabled:cursor-not-allowed disabled:opacity-50"
				style="border-color: color-mix(in srgb, var(--text-muted) 28%, transparent)"
				placeholder="end"
				value={row.node.rangeEnd}
				disabled={actionsDisabled}
				oninput={(event) =>
					onUpdateRange(row.id, "rangeEnd", (event.currentTarget as HTMLInputElement).value)}
			/>
			<input
				id={`ns-${row.id}-rangeStep`}
				name={`ns-${row.id}-rangeStep`}
				class="w-[62px] rounded-md border bg-(--bg-muted) px-1 py-0.5 text-[11px] text-(--text-primary) disabled:cursor-not-allowed disabled:opacity-50"
				style="border-color: color-mix(in srgb, var(--text-muted) 28%, transparent)"
				placeholder="step"
				value={row.node.rangeStep}
				disabled={actionsDisabled}
				oninput={(event) =>
					onUpdateRange(row.id, "rangeStep", (event.currentTarget as HTMLInputElement).value)}
			/>
			{#if row.node.children.length > 0}
				<span
					class="ml-4 inline-flex items-center rounded-full bg-blue-900 px-2 py-0.5 text-[10px] text-blue-100"
					>Folder</span>
			{:else}
				<div class="ml-4 inline-flex flex-wrap gap-0.5">
					{#each allowedDataTypes as dataType}
						<Button
							variant="outline-muted"
							label={dataType}
							title={dataType}
							selected={row.node.dataType === dataType}
							disabled={actionsDisabled}
							onclick={() => onUpdateType(row.id, dataType)}
							class="rounded-full px-2 py-0.5 text-[10px]"
						/>
					{/each}
				</div>
			{/if}
		</div>
	</div>
</div>

<style>
	.ns-drop-before {
		box-shadow: inset 0 2px 0 0 #2563eb;
	}
	.ns-drop-child {
		box-shadow: inset 0 -2px 0 0 #2563eb;
		background: color-mix(in srgb, #2563eb 10%, transparent);
	}
	.ns-drop-forbidden {
		cursor: not-allowed;
		background: color-mix(in srgb, var(--text-muted) 22%, transparent);
		box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--text-muted) 45%, transparent);
		opacity: 0.72;
	}
	.tree-indent-guides {
		position: absolute;
		left: 0;
		top: 0;
		height: 100%;
		pointer-events: none;
		z-index: 0;
		--tree-guide: color-mix(in srgb, var(--text-muted) 32%, transparent);
	}
	.tree-indent-guide-vertical {
		position: absolute;
		top: -4px;
		height: calc(100% + 4px);
		width: 1px;
		background: var(--tree-guide);
	}
	.tree-indent-guide-branch {
		position: absolute;
		top: 50%;
		height: 12px;
		margin-top: -6px;
		overflow: visible;
		pointer-events: none;
		color: color-mix(in srgb, var(--text-muted) 48%, transparent);
		opacity: 0.9;
	}
	.tree-indent-guide-branch-path {
		stroke: currentColor;
		stroke-width: 1;
		stroke-linecap: round;
		stroke-linejoin: round;
	}
	.ns-node-content > :not(.tree-indent-guides) {
		position: relative;
		z-index: 1;
	}
</style>
