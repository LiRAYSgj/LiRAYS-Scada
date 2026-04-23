<script lang="ts">
  import { Checkbox } from "$lib/components/ui/checkbox";
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
    onCheckClick?: () => void;
    /** Emits hover metadata anchor for tag rows. */
    onTagTooltipChange?: (
      payload: { node: TreeNode; anchorRect: DOMRect } | null,
    ) => void;
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
    onTagTooltipChange,
  }: Props = $props();

  let rowEl: HTMLDivElement | null = null;
  const displayValue = $derived(
    liveValue !== undefined ? String(liveValue) : (row.node.value ?? "-"),
  );

  function showTagTooltip(): void {
    if (row.node.kind !== "tag" || !rowEl || !onTagTooltipChange) {
      return;
    }
    onTagTooltipChange({
      node: row.node,
      anchorRect: rowEl.getBoundingClientRect(),
    });
  }

  function hideTagTooltip(): void {
    onTagTooltipChange?.(null);
  }
</script>

<div
  bind:this={rowEl}
  class={`group relative grid h-8 grid-cols-[1fr_90px_90px_80px] items-center border-b border-border px-2 text-xs ${
    multiSelectMode ? "cursor-default" : "cursor-pointer"
  } ${!multiSelectMode && isSelected ? "bg-primary/15" : "hover:bg-muted"}`}
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
  onmouseenter={showTagTooltip}
  onmouseleave={hideTagTooltip}
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
      <Checkbox
        checked={isChecked}
        indeterminate={isIndeterminate}
        aria-label={isIndeterminate ? "Partially selected" : isChecked ? "Deselect" : "Select"}
        class="cursor-pointer"
        onclick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          onCheckClick?.();
        }}
      />
    {/if}
    <TreeIcon kind={row.node.kind} />
    <span class="truncate text-foreground">{row.node.name}</span>
    {#if row.isErrored}
      <span
        class="ml-2 rounded bg-red-200 px-1.5 py-0.5 text-[10px] text-red-700 dark:bg-red-950 dark:text-red-300"
      >
        load error
      </span>
    {/if}
  </div>
  <span class="truncate text-muted-foreground"
    >{formatDataType(row.node.dataType)}</span
  >
  <span class="truncate text-muted-foreground">{displayValue}</span>
  <span class="truncate text-muted-foreground">{row.node.unit ?? "-"}</span>
</div>
