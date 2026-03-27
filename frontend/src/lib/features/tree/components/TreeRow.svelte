<script lang="ts">
  import TreeChevron from "./TreeChevron.svelte";
  import TreeIcon from "./TreeIcon.svelte";
  import type { TreeNode, VisibleTreeRow } from "../types";
  import type { TagScalarValue } from "$lib/core/ws/types";

  function formatDataType(dataType?: string): string {
    if (!dataType) return "-";
    if (dataType === "VAR_DATA_TYPE_INTEGER") return "Integer";
    if (dataType === "VAR_DATA_TYPE_FLOAT") return "Float";
    if (dataType === "VAR_DATA_TYPE_TEXT") return "Text";
    if (dataType === "VAR_DATA_TYPE_BOOLEAN") return "Bool";
    return dataType;
  }

  interface Props {
    row: VisibleTreeRow;
    isSelected: boolean;
    onSelect: () => void;
    onToggle: () => void;
    onContextMenu: (event: MouseEvent, node: TreeNode) => void;
    onDragStart: (event: DragEvent, node: TreeNode) => void;
    onDragEnd: (event: DragEvent) => void;
    liveValue?: TagScalarValue;
    /** When true, show selection checkbox and disable single-select/drag/context menu. */
    multiSelectMode?: boolean;
    /** Whether this node is in the multi-selection (only when multiSelectMode). */
    isChecked?: boolean;
    /** Partially selected: some but not all loaded children are selected (only when multiSelectMode). */
    isIndeterminate?: boolean;
    /** Called when the selection checkbox is clicked (only when multiSelectMode). */
    onCheckClick?: (event: MouseEvent) => void;
  }

  let {
    row,
    isSelected,
    onSelect,
    onToggle,
    onContextMenu,
    onDragStart,
    onDragEnd,
    liveValue,
    multiSelectMode = false,
    isChecked = false,
    isIndeterminate = false,
    onCheckClick,
  }: Props = $props();

  const displayValue = $derived(
    liveValue !== undefined ? String(liveValue) : (row.node.value ?? "-"),
  );
</script>

<div
  class={`group relative grid h-8 grid-cols-[1fr_90px_90px_80px] items-center border-b border-black/10 px-2 text-xs dark:border-white/10 ${
    multiSelectMode ? "cursor-default" : "cursor-pointer"
  } ${!multiSelectMode && isSelected ? "bg-(--bg-selected)" : "hover:bg-(--bg-hover)"}`}
  role="treeitem"
  tabindex="-1"
  aria-expanded={row.node.hasChildren ? row.isExpanded : undefined}
  aria-selected={multiSelectMode ? undefined : isSelected}
  aria-level={row.depth}
  draggable={!multiSelectMode}
  onclick={multiSelectMode ? undefined : onSelect}
  oncontextmenu={(event) => {
    event.preventDefault();
    if (!multiSelectMode) onContextMenu(event, row.node);
  }}
  ondragstart={multiSelectMode ? undefined : (event) => onDragStart(event, row.node)}
  ondragend={multiSelectMode ? undefined : onDragEnd}
  onkeydown={(event) => {
    if (multiSelectMode) return;
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      onSelect();
    }
  }}
>
  <div class="flex items-center gap-1 overflow-hidden">
    <div
      style={`width: ${(row.depth - 1) * 14}px`}
      class="shrink-0"
      aria-hidden="true"
    ></div>
    <TreeChevron
      hasChildren={row.node.hasChildren}
      isExpanded={row.isExpanded}
      isLoading={row.isLoading}
      {onToggle}
    />
    {#if multiSelectMode}
      <button
        type="button"
        class="cursor-pointer flex h-5 w-5 shrink-0 items-center justify-center rounded border border-black/25 bg-(--bg-panel) text-(--text-primary) hover:border-black/40 dark:border-white/25 dark:hover:border-white/40"
        role="checkbox"
        aria-label={isIndeterminate ? "Partially selected" : isChecked ? "Deselect" : "Select"}
        aria-checked={isChecked ? "true" : isIndeterminate ? "mixed" : "false"}
        onclick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          onCheckClick?.(e);
        }}
      >
        {#if isIndeterminate}
          <svg class="h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <line x1="5" y1="12" x2="19" y2="12" />
          </svg>
        {:else if isChecked}
          <svg class="h-3 w-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="20 6 9 17 4 12" />
          </svg>
        {/if}
      </button>
    {/if}
    <TreeIcon kind={row.node.kind} />
    <span class="truncate text-(--text-primary)">{row.node.name}</span>
    {#if row.isErrored}
      <span
        class="ml-2 rounded bg-red-200 px-1.5 py-0.5 text-[10px] text-red-700 dark:bg-red-950 dark:text-red-300"
      >
        load error
      </span>
    {/if}
  </div>
  <span class="truncate text-(--text-muted)"
    >{formatDataType(row.node.dataType)}</span
  >
  <span class="truncate text-(--text-secondary)">{displayValue}</span>
  <span class="truncate text-(--text-secondary)">{row.node.unit ?? "-"}</span>

  {#if row.node.kind === "tag"}
    <div
      class="pointer-events-none absolute left-2 top-full z-10 mt-1 hidden min-w-[220px] rounded-md border border-black/10 bg-(--bg-panel) px-3 py-2 text-[11px] shadow-lg transition-opacity duration-150 group-hover:block dark:border-white/10"
    >
      <div class="space-y-1 text-(--text-secondary)">
        <div class="flex justify-between gap-2">
          <span class="text-(--text-muted)">Unit</span>
          <span>{row.node.unit ?? "-"}</span>
        </div>
        {#if row.node.dataType === "VAR_DATA_TYPE_INTEGER" || row.node.dataType === "VAR_DATA_TYPE_FLOAT"}
          <div class="flex justify-between gap-2">
            <span class="text-(--text-muted)">Min</span>
            <span>{row.node.min ?? "-"}</span>
          </div>
          <div class="flex justify-between gap-2">
            <span class="text-(--text-muted)">Max</span>
            <span>{row.node.max ?? "-"}</span>
          </div>
        {/if}
        {#if row.node.dataType === "VAR_DATA_TYPE_TEXT"}
          <div class="flex justify-between gap-2">
            <span class="text-(--text-muted)">Max len</span>
            <span>{row.node.maxLen && row.node.maxLen.length > 0 ? row.node.maxLen[0] : "-"}</span>
          </div>
          <div>
            <div class="text-(--text-muted)">Options</div>
            <div class="truncate text-(--text-secondary)">
              {row.node.options && row.node.options.length
                ? row.node.options.join(", ")
                : "-"}
            </div>
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>
