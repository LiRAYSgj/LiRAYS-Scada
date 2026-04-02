<script lang="ts">
	import { Button } from "$lib/components/Button";
	import { Input } from "$lib/components/ui/input";
	import { NumberField } from "$lib/components/ui/number-field";
	import { TagsInput } from "$lib/components/ui/tags-input";
	import * as Select from "$lib/components/ui/select";
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
		onUpdateSeriesMode,
		onUpdateType,
		onUpdateVariableMeta
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
			key: "rangeStart" | "rangeEnd" | "rangeStep" | "nameSuffix" | "seriesValues",
			value: string
		) => void;
		onUpdateSeriesMode: (rowId: string, mode: "literal" | "range" | "set") => void;
		onUpdateType: (rowId: string, dataType: string) => void;
		onUpdateVariableMeta: (
			rowId: string,
			key: "unit" | "min" | "max" | "maxLength" | "options",
			value: string | string[]
		) => void;
	} = $props();

	let rowElement: HTMLDivElement | null = null;

	/** Single indent step (px per depth level); padding and guides use this so indent is linear. */
	const INDENT_STEP = 18;
	const nodeNamePlaceholder = "Node Name";
	const generatedRangePlaceholder = "Generated based on range";
	const generatedSetPlaceholder = "Generated based on set";

	function hasGeneratedSeries(): boolean {
		if (row.node.seriesMode === "numeric") {
			return row.node.rangeEnd.trim().length > 0;
		}
		if (row.node.seriesMode === "enum") {
			return (
				row.node.seriesValues
					.split(",")
					.map((value) => value.trim())
					.filter((value) => value.length > 0).length > 0
			);
		}
		return false;
	}

	function getSeriesSelectValue(): "literal" | "range" | "set" {
		if (row.node.seriesMode === "numeric") return "range";
		if (row.node.seriesMode === "enum") return "set";
		return "literal";
	}

	function getSeriesSelectLabel(): "Literal" | "Range" | "Set" {
		const value = getSeriesSelectValue();
		if (value === "range") return "Range";
		if (value === "set") return "Set";
		return "Literal";
	}

	function toEnumTags(raw: string): string[] {
		return raw
			.split(",")
			.map((value) => value.trim())
			.filter((value) => value.length > 0);
	}

	function displayNodeLabel(): string {
		if (row.node.name.trim().length > 0) return row.node.name;
		if (row.node.seriesMode === "numeric" && hasGeneratedSeries()) {
			return generatedRangePlaceholder;
		}
		if (row.node.seriesMode === "enum" && hasGeneratedSeries()) {
			return generatedSetPlaceholder;
		}
		return nodeNamePlaceholder;
	}

	function getNameEditPlaceholder(): string {
		if (row.node.seriesMode === "numeric" && hasGeneratedSeries()) {
			return generatedRangePlaceholder;
		}
		if (row.node.seriesMode === "enum" && hasGeneratedSeries()) {
			return generatedSetPlaceholder;
		}
		return nodeNamePlaceholder;
	}

	function getSelectPortalTarget(): Element | string {
		if (typeof document === "undefined") return "body";
		const dialog = rowElement?.closest("dialog[open]");
		if (dialog instanceof Element) return dialog;
		const anyOpenDialog = document.querySelector("dialog[open]");
		if (anyOpenDialog instanceof Element) return anyOpenDialog;
		return "body";
	}

	function getDataTypeKey(): "float" | "integer" | "text" | "boolean" | "other" {
		const raw = (row.node.dataType ?? "").toLowerCase();
		if (raw === "float") return "float";
		if (raw === "integer" || raw === "int") return "integer";
		if (raw === "text" || raw === "string") return "text";
		if (raw === "boolean" || raw === "bool") return "boolean";
		return "other";
	}

	function isNumericType(): boolean {
		const key = getDataTypeKey();
		return key === "float" || key === "integer";
	}

	function isTextType(): boolean {
		return getDataTypeKey() === "text";
	}

	function toOptionTags(raw: string[]): string[] {
		return raw
			.map((value) => value.trim())
			.filter((value) => value.length > 0);
	}

	function parseOptionalNumber(raw: string): number | undefined {
		const value = raw.trim();
		if (value === "") return undefined;
		const parsed = Number(value);
		return Number.isFinite(parsed) ? parsed : undefined;
	}
</script>

<div
	bind:this={rowElement}
	class={`group/row box-border flex min-h-9 w-full items-center border-b transition-opacity ${draggedNodeId === row.id ? "opacity-[0.35]" : ""} ${
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
		class="ns-node-content relative flex w-full min-w-0 flex-nowrap items-center gap-2.5 py-1 pr-2 pl-0"
		style={`padding-left:${row.depth * INDENT_STEP + 6}px`}
	>
		{#if row.depth > 0}
			<div
				class="tree-indent-guides"
				style={`width:${row.depth * INDENT_STEP}px`}
				aria-hidden="true"
			>
				{#each Array(row.depth) as _, i}
					<span
						class="tree-indent-guide-vertical"
						style={`left:${i * INDENT_STEP}px`}
					></span>
				{/each}
				<svg
					class="tree-indent-guide-branch"
					style={`left:${(row.depth - 1) * INDENT_STEP}px;width:${INDENT_STEP}px`}
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
			<Input
				id={`ns-name-${row.id}`}
				name={`ns-name-${row.id}`}
				bind:ref={editingInputEl}
				class="min-w-[180px] max-w-[320px] h-7 text-[13px] border-blue-600"
				bind:value={editingName}
				placeholder={getNameEditPlaceholder()}
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
				label={displayNodeLabel()}
				disabled={actionsDisabled}
				ondblclick={() => !actionsDisabled && onStartEditName(row.id)}
				class={`btn--align-start min-w-[150px] py-0 text-[13px] cursor-text h-[22px] ${
					row.node.name.trim().length === 0
						? "text-(--text-muted) opacity-70"
						: ""
				}`}
			/>
		{/if}
		<div class="flex min-w-0 flex-nowrap items-center gap-1.5 [&>*]:shrink-0">
			<Select.Root
				type="single"
				value={getSeriesSelectValue()}
				onValueChange={(value) =>
					onUpdateSeriesMode(row.id, (value as "literal" | "range" | "set") ?? "literal")}
				disabled={actionsDisabled}
			>
				<Select.Trigger class="w-[120px]">{getSeriesSelectLabel()}</Select.Trigger>
				<Select.Content
					portalProps={{ to: getSelectPortalTarget() }}
					class="z-[2147483647] border border-black/15 bg-(--bg-panel) shadow-lg dark:border-white/10"
					style="background-color: var(--bg-panel);"
				>
					<Select.Group>
						<Select.Item value="literal" label="Literal" />
						<Select.Item value="range" label="Range" />
						<Select.Item value="set" label="Set" />
					</Select.Group>
				</Select.Content>
			</Select.Root>
			{#if row.node.seriesMode === "numeric"}
				<Input
					id={`ns-${row.id}-rangeStart`}
					name={`ns-${row.id}-rangeStart`}
					class="w-[62px] h-7 text-[11px]"
					placeholder="Start"
					value={row.node.rangeStart}
					disabled={actionsDisabled}
					oninput={(event) =>
						onUpdateRange(row.id, "rangeStart", (event.currentTarget as HTMLInputElement).value)}
				/>
				<Input
					id={`ns-${row.id}-rangeEnd`}
					name={`ns-${row.id}-rangeEnd`}
					class="w-[62px] h-7 text-[11px]"
					placeholder="End"
					value={row.node.rangeEnd}
					disabled={actionsDisabled}
					oninput={(event) =>
						onUpdateRange(row.id, "rangeEnd", (event.currentTarget as HTMLInputElement).value)}
				/>
				<Input
					id={`ns-${row.id}-rangeStep`}
					name={`ns-${row.id}-rangeStep`}
					class="w-[62px] h-7 text-[11px]"
					placeholder="Step"
					value={row.node.rangeStep}
					disabled={actionsDisabled}
					oninput={(event) =>
						onUpdateRange(row.id, "rangeStep", (event.currentTarget as HTMLInputElement).value)}
				/>
			{:else if row.node.seriesMode === "enum"}
				<div class="w-[260px]">
					<TagsInput
						id={`ns-${row.id}-seriesValues`}
						class="w-full h-8 min-h-8 max-h-8 rounded-lg border border-black/15 bg-(--bg-muted) text-xs !flex-nowrap overflow-x-auto overflow-y-hidden [scrollbar-width:thin] [&>div]:shrink-0 [&_input]:min-w-[96px] [&_input]:shrink-0 [&_input]:grow-0 dark:border-white/10"
						value={toEnumTags(row.node.seriesValues)}
						onValueChange={(values) => onUpdateRange(row.id, "seriesValues", values.join(", "))}
						commitSeparators={[","]}
						placeholder="Add values"
						disabled={actionsDisabled}
					/>
				</div>
			{/if}
			{#if row.node.seriesMode !== "none"}
				<Input
					id={`ns-${row.id}-nameSuffix`}
					name={`ns-${row.id}-nameSuffix`}
					class="w-[80px] h-7 text-[11px]"
					placeholder="Suffix"
					value={row.node.nameSuffix}
					disabled={actionsDisabled}
					oninput={(event) =>
						onUpdateRange(row.id, "nameSuffix", (event.currentTarget as HTMLInputElement).value)}
				/>
			{/if}
			{#if row.node.children.length > 0}
				<span
					class="ml-4 inline-flex items-center rounded-full bg-blue-900 px-2 py-0.5 text-[10px] text-blue-100"
					>Folder</span>
			{:else}
				<div class="ml-4 inline-flex flex-nowrap gap-0.5">
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
				{#if isNumericType()}
					<Input
						id={`ns-${row.id}-unit`}
						name={`ns-${row.id}-unit`}
						class="w-[90px] h-7 text-[11px]"
						placeholder="Unit"
						value={row.node.unit}
						disabled={actionsDisabled}
						oninput={(event) =>
							onUpdateVariableMeta(row.id, "unit", (event.currentTarget as HTMLInputElement).value)}
					/>
					<NumberField
						id={`ns-${row.id}-min`}
						name={`ns-${row.id}-min`}
						class="w-[94px] h-7 text-[11px]"
						placeholder="Min"
						value={parseOptionalNumber(row.node.min)}
						step="any"
						disabled={actionsDisabled}
						onValueChange={(value) =>
							onUpdateVariableMeta(row.id, "min", value === undefined ? "" : String(value))}
					/>
					<NumberField
						id={`ns-${row.id}-max`}
						name={`ns-${row.id}-max`}
						class="w-[94px] h-7 text-[11px]"
						placeholder="Max"
						value={parseOptionalNumber(row.node.max)}
						step="any"
						disabled={actionsDisabled}
						onValueChange={(value) =>
							onUpdateVariableMeta(row.id, "max", value === undefined ? "" : String(value))}
					/>
				{:else if isTextType()}
					<Input
						id={`ns-${row.id}-maxLength`}
						name={`ns-${row.id}-maxLength`}
						class="w-[88px] h-7 text-[11px]"
						placeholder="Max Length"
						value={row.node.maxLength}
						disabled={actionsDisabled}
						oninput={(event) =>
							onUpdateVariableMeta(row.id, "maxLength", (event.currentTarget as HTMLInputElement).value)}
					/>
					<div class="w-[220px]">
						<TagsInput
							id={`ns-${row.id}-options`}
							class="w-full h-8 min-h-8 max-h-8 rounded-lg border border-black/15 bg-(--bg-muted) text-xs !flex-nowrap overflow-x-auto overflow-y-hidden [scrollbar-width:thin] [&>div]:shrink-0 [&_input]:min-w-[88px] [&_input]:shrink-0 [&_input]:grow-0 dark:border-white/10"
							value={toOptionTags(row.node.options)}
							onValueChange={(values) => onUpdateVariableMeta(row.id, "options", values)}
							commitSeparators={[","]}
							placeholder="Options"
							disabled={actionsDisabled}
						/>
					</div>
				{/if}
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
