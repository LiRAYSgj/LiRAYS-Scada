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
  }: Props = $props();

  const displayValue = $derived(
    liveValue !== undefined ? String(liveValue) : (row.node.value ?? "-"),
  );
</script>

<div
  class={`grid h-8 cursor-pointer grid-cols-[1fr_90px_90px] items-center border-b border-black/10 px-2 text-xs dark:border-white/10 ${
    isSelected ? "bg-(--bg-selected)" : "hover:bg-(--bg-hover)"
  }`}
  role="treeitem"
  tabindex="-1"
  aria-expanded={row.node.hasChildren ? row.isExpanded : undefined}
  aria-selected={isSelected}
  aria-level={row.depth}
  draggable="true"
  onclick={onSelect}
  oncontextmenu={(event) => {
    event.preventDefault();
    onContextMenu(event, row.node);
  }}
  ondragstart={(event) => onDragStart(event, row.node)}
  ondragend={onDragEnd}
  onkeydown={(event) => {
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
  <span class="truncate text-(--text-secondary)">{displayValue}</span>
  <span class="truncate text-(--text-muted)"
    >{formatDataType(row.node.dataType)}</span
  >
</div>
