<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { get } from "svelte/store";
  import { Circle, LoaderCircle } from "lucide-svelte";
  import TreeRow from "./TreeRow.svelte";
  import TagMetadataTooltip from "./TagMetadataTooltip.svelte";
  import VariableTreeAddDialog from "./VariableTreeAddDialog.svelte";
  import VariableTreeEditDialog from "./VariableTreeEditDialog.svelte";
  import { fetchTreeChildren } from "../server-adapter";
  import { createTreeStore } from "../tree-store";
  import { readTreeStateCache, writeTreeStateCache } from "../tree-cache";
  import {
    getLoadedDescendantIds,
  } from "../tree-selection";
  import { buildTreeSelectionSummary } from "../tree-selection-summary";
  import type { TreeNode } from "../types";
  import {
    type TagScalarValue,
    WebSocketConnectionStatus,
  } from "$lib/core/ws/types";
  import { tagStreamClient } from "$lib/core/ws/tag-stream-client";
  import { ItemType, type VarDataType } from "@lirays/scada-proto";
  import type { VarMetaChanged } from "@lirays/scada-proto";

  export interface SelectionChangePayload {
    add: string[];
    remove: string[];
  }

  interface Props {
    onNodeContextMenu: (event: MouseEvent, node: TreeNode) => void;
    onNodeDragStart: (event: DragEvent, node: TreeNode) => void;
    onNodeDragEnd: (event: DragEvent) => void;
    websocketStatus?: WebSocketConnectionStatus;
    realtimeEnabled?: boolean;
    liveTagValues?: Record<string, TagScalarValue>;
    onRootId?: (id: string | null) => void;
    onCreateItem: (input: {
      parentId: string | null;
      name: string;
      itemType: ItemType;
      varType: VarDataType | undefined;
      unit?: string;
      min?: number;
      max?: number;
      options?: string[];
      maxLen?: number;
    }) => Promise<void>;
    onEditMeta: (input: {
      varId: string;
      unit?: string;
      min?: number;
      max?: number;
      options?: string[];
      maxLen?: number;
    }) => Promise<void>;
    multiSelectMode?: boolean;
    selection?: Set<string>;
    propagateDown?: boolean;
    propagateUp?: boolean;
    onSelectionChange?: (payload: SelectionChangePayload) => void;
    onTreeStateSnapshot?: (
      nodes: Record<string, TreeNode>,
      rootIds: string[],
    ) => void;
    onVarMetaChanged?: (changes: VarMetaChanged[]) => void;
  }

  let {
    onNodeContextMenu,
    onNodeDragStart,
    onNodeDragEnd,
    websocketStatus = WebSocketConnectionStatus.DISCONNECTED,
    realtimeEnabled = false,
    liveTagValues = {},
    onRootId,
    onCreateItem,
    onEditMeta,
    multiSelectMode = false,
    selection = new SvelteSet<string>(),
    propagateDown = true,
    propagateUp = true,
    onSelectionChange,
    onTreeStateSnapshot,
    onVarMetaChanged,
  }: Props = $props();

  const tree = createTreeStore({
    fetchChildren: fetchTreeChildren,
  });
  const treeState = tree.state;
  const visibleRows = tree.visibleRows;
  const skeletonRows = Array.from(
    { length: 14 },
    (_, index) => `skeleton-${index}`,
  );

  const ROW_HEIGHT = 32;
  const OVERSCAN = 10;
  const TAG_TOOLTIP_WIDTH = 240;
  const TAG_TOOLTIP_OFFSET_X = 12;
  const TAG_TOOLTIP_MARGIN = 8;
  const TAG_TOOLTIP_MAX_HEIGHT = 220;

  interface TagTooltipState {
    node: TreeNode;
    x: number;
    y: number;
  }

  let treeViewportEl: HTMLDivElement | null = null;
  let tagTooltip = $state<TagTooltipState | null>(null);
  let scrollTop = $state(0);
  let viewportHeight = $state(0);
  let addDialogOpen = $state(false);
  let addDialogParentId = $state<string | null>(null);
  let editDialogOpen = $state(false);
  let editDialogNode = $state<TreeNode | null>(null);
  let retriedRootLoadOnConnected = $state(false);

  const isConnected = $derived(
    websocketStatus === WebSocketConnectionStatus.CONNECTED,
  );
  const totalRows = $derived($visibleRows.length);
  const startIndex = $derived(
    Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN),
  );
  const visibleCount = $derived(
    Math.max(1, Math.ceil(viewportHeight / ROW_HEIGHT) + OVERSCAN * 2),
  );
  const endIndex = $derived(Math.min(totalRows, startIndex + visibleCount));
  const windowRows = $derived($visibleRows.slice(startIndex, endIndex));
  const topPadding = $derived(startIndex * ROW_HEIGHT);
  const bottomPadding = $derived((totalRows - endIndex) * ROW_HEIGHT);
  const selectionSummary = $derived(
    buildTreeSelectionSummary($treeState.nodes, $treeState.rootIds, selection),
  );

  onMount(() => {
    const cachedState = readTreeStateCache();
    if (cachedState) {
      tree.state.set(cachedState);
    }
    void tree.initialize();

    const unsubTree = tagStreamClient.treeChanges.subscribe((ev) => {
      if (ev?.folderChangedEvent?.length) {
        void tree.applyRemoteTreeChanged(ev);
      } else if (ev?.varMetaChangedEvent?.length) {
        void tree.applyRemoteTreeChanged(ev);
      }
      if (ev?.varMetaChangedEvent?.length) {
        onVarMetaChanged?.([...ev.varMetaChangedEvent]);
      }
    });
    const unsubCache = tree.state.subscribe((snapshot) => {
      writeTreeStateCache(snapshot);
    });

    if (!treeViewportEl) {
      return () => {
        unsubTree();
        unsubCache();
      };
    }

    viewportHeight = treeViewportEl.clientHeight;
    const observer = new ResizeObserver(() => {
      if (!treeViewportEl) {
        return;
      }
      viewportHeight = treeViewportEl.clientHeight;
    });
    observer.observe(treeViewportEl);

    return () => {
      unsubTree();
      unsubCache();
      observer.disconnect();
    };
  });

  $effect(() => {
    if (!onRootId) return;
    const state = $treeState;
    if (state.rootIds.length > 0) {
      onRootId(state.rootIds[0]);
    } else if (state.hasInitialized) {
      onRootId(null);
    }
  });

  $effect(() => {
    if (!onTreeStateSnapshot) return;
    const state = $treeState;
    onTreeStateSnapshot(state.nodes, state.rootIds);
  });

  $effect(() => {
    const status = websocketStatus;
    const state = $treeState;

    if (status !== WebSocketConnectionStatus.CONNECTED) {
      retriedRootLoadOnConnected = false;
      return;
    }
    if (state.rootLoading || state.rootIds.length > 0) {
      return;
    }
    if (retriedRootLoadOnConnected) {
      return;
    }

    retriedRootLoadOnConnected = true;
    void tree.initialize();
  });

  $effect(() => {
    if (!multiSelectMode || !propagateDown || !onSelectionChange) return;
    const state = $treeState;
    const nodes = state.nodes;
    const sel = untrack(() => selection);
    const toAdd: string[] = [];
    const seenToAdd: Record<string, true> = {};
    for (const selectedId of sel) {
      for (const id of getLoadedDescendantIds(selectedId, nodes)) {
        if (!sel.has(id) && !seenToAdd[id]) {
          seenToAdd[id] = true;
          toAdd.push(id);
        }
      }
    }
    if (toAdd.length > 0) {
      onSelectionChange({ add: toAdd, remove: [] });
    }
  });

  function ensureIndexVisible(index: number): void {
    if (!treeViewportEl) {
      return;
    }

    const rowTop = index * ROW_HEIGHT;
    const rowBottom = rowTop + ROW_HEIGHT;
    const viewportTop = treeViewportEl.scrollTop;
    const viewportBottom = viewportTop + treeViewportEl.clientHeight;

    if (rowTop < viewportTop) {
      treeViewportEl.scrollTop = rowTop;
    } else if (rowBottom > viewportBottom) {
      treeViewportEl.scrollTop = rowBottom - treeViewportEl.clientHeight;
    }
  }

  function selectNode(nodeId: string): void {
    tree.selectNode(nodeId);
  }

  function toggleNode(nodeId: string): void {
    tree.toggleExpanded(nodeId);
  }

  function handleTreeKeyDown(event: KeyboardEvent): void {
    const rows = get(visibleRows);
    const currentState = get(treeState);
    if (rows.length === 0) {
      return;
    }

    const selectedId = currentState.selectedId ?? rows[0].id;
    const selectedIndex = Math.max(
      0,
      rows.findIndex((row) => row.id === selectedId),
    );
    const selectedRow = rows[selectedIndex];

    if (event.key === "ArrowDown") {
      event.preventDefault();
      const nextIndex = Math.min(rows.length - 1, selectedIndex + 1);
      const next = rows[nextIndex];
      tree.selectNode(next.id);
      ensureIndexVisible(nextIndex);
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      const prevIndex = Math.max(0, selectedIndex - 1);
      const prev = rows[prevIndex];
      tree.selectNode(prev.id);
      ensureIndexVisible(prevIndex);
      return;
    }

    if (event.key === "ArrowRight") {
      event.preventDefault();
      if (selectedRow.node.hasChildren && !selectedRow.isExpanded) {
        tree.toggleExpanded(selectedRow.id);
        return;
      }

      const childId = selectedRow.node.childIds?.[0];
      if (childId) {
        tree.selectNode(childId);
      }
      return;
    }

    if (event.key === "ArrowLeft") {
      event.preventDefault();
      if (selectedRow.isExpanded) {
        tree.collapseNode(selectedRow.id);
        return;
      }

      if (selectedRow.node.parentId) {
        tree.selectNode(selectedRow.node.parentId);
      }
      return;
    }

    if (event.key === "Home") {
      event.preventDefault();
      tree.selectNode(rows[0].id);
      ensureIndexVisible(0);
      return;
    }

    if (event.key === "End") {
      event.preventDefault();
      tree.selectNode(rows[rows.length - 1].id);
      ensureIndexVisible(rows.length - 1);
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      tree.selectNode(selectedRow.id);
    }
  }

  function resolveParentIdForCreate(): string | null {
    const snapshot = get(treeState);
    const selected = snapshot.selectedId
      ? snapshot.nodes[snapshot.selectedId]
      : null;
    if (selected?.kind === "folder") {
      return selected.id;
    }
    if (selected?.kind === "tag") {
      return selected.parentId;
    }
    return snapshot.rootIds[0] ?? null;
  }

  export function openCreateDialog(parentId?: string | null): void {
    if (!isConnected) {
      return;
    }
    addDialogParentId =
      parentId === undefined ? resolveParentIdForCreate() : parentId;
    addDialogOpen = true;
  }

  export function openMetadataDialog(node: TreeNode): void {
    if (!isConnected || node.kind !== "tag") {
      return;
    }
    editDialogNode = node;
    editDialogOpen = true;
  }

  function handleSelectionCheckClick(nodeId: string): void {
    if (!onSelectionChange) return;
    const state = get(treeState);
    const nodes = state.nodes;
    const currentlyChecked = selection.has(nodeId);
    const newChecked = !currentlyChecked;

    const add: string[] = [];
    const remove: string[] = [];

    if (newChecked) {
      add.push(nodeId);
      if (propagateDown) {
        add.push(...getLoadedDescendantIds(nodeId, nodes));
      }
      if (propagateUp) {
        let parentId: string | null = nodes[nodeId]?.parentId ?? null;
        while (parentId) {
          const parent = nodes[parentId];
          if (!parent?.childIds) break;
          const allSiblingsSelected = parent.childIds.every((id) =>
            id === nodeId || selection.has(id) || add.includes(id),
          );
          if (!allSiblingsSelected) break;
          add.push(parentId);
          parentId = parent.parentId ?? null;
        }
      }
    } else {
      remove.push(nodeId);
      if (propagateDown) {
        remove.push(...getLoadedDescendantIds(nodeId, nodes));
      }
    }

    onSelectionChange({ add, remove });
  }

  function clamp(value: number, min: number, max: number): number {
    return Math.min(Math.max(value, min), max);
  }

  function hideTagTooltip(): void {
    tagTooltip = null;
  }

  function handleTagTooltipChange(
    payload: { node: TreeNode; anchorRect: DOMRect } | null,
  ): void {
    if (!payload) {
      hideTagTooltip();
      return;
    }

    const maxX = Math.max(
      TAG_TOOLTIP_MARGIN,
      window.innerWidth - TAG_TOOLTIP_WIDTH - TAG_TOOLTIP_MARGIN,
    );
    const maxY = Math.max(
      TAG_TOOLTIP_MARGIN,
      window.innerHeight - TAG_TOOLTIP_MAX_HEIGHT - TAG_TOOLTIP_MARGIN,
    );
    tagTooltip = {
      node: payload.node,
      x: clamp(
        payload.anchorRect.right + TAG_TOOLTIP_OFFSET_X,
        TAG_TOOLTIP_MARGIN,
        maxX,
      ),
      y: clamp(payload.anchorRect.top, TAG_TOOLTIP_MARGIN, maxY),
    };
  }
</script>

<section class="flex h-full flex-col rounded-md border border-border bg-card">
  <header
    class="grid h-9 grid-cols-[1fr_90px_90px_80px] items-center border-b border-border px-2 text-[11px] tracking-wider text-muted-foreground uppercase"
  >
    <span> </span>
    <span>Type</span>
    <span>Value</span>
    <span>Unit</span>
  </header>

  <div
    class="flex-1 overflow-auto"
    bind:this={treeViewportEl}
    onscroll={() => {
      if (!treeViewportEl) {
        return;
      }
      hideTagTooltip();
      scrollTop = treeViewportEl.scrollTop;
    }}
  >
    {#if !$treeState.hasInitialized || $treeState.rootLoading}
      <div class="space-y-2 p-2">
        {#each skeletonRows as rowKey (rowKey)}
          <div
            class="grid h-8 grid-cols-[1fr_90px_90px_80px] items-center gap-2 rounded px-2"
          >
            <div class="h-3.5 animate-pulse rounded bg-muted/50"></div>
            <div class="h-3.5 animate-pulse rounded bg-muted/50"></div>
            <div class="h-3.5 animate-pulse rounded bg-muted/50"></div>
            <div class="h-3.5 animate-pulse rounded bg-muted/50"></div>
          </div>
        {/each}
      </div>
    {:else if $visibleRows.length === 0}
      <div class="p-3 text-xs text-muted-foreground">No nodes available</div>
    {:else}
      <div
        role="tree"
        tabindex="0"
        class="outline-none"
        onkeydown={handleTreeKeyDown}
      >
        <div style={`height: ${topPadding}px`} aria-hidden="true"></div>
        {#each windowRows as row (row.id)}
          <TreeRow
            {row}
            isSelected={$treeState.selectedId === row.id}
            onSelect={() => selectNode(row.id)}
            onToggle={() => toggleNode(row.id)}
            onContextMenu={onNodeContextMenu}
            onDragStart={onNodeDragStart}
            onDragEnd={onNodeDragEnd}
            liveValue={realtimeEnabled && row.node.kind === "tag"
              ? liveTagValues[row.node.id]
              : undefined}
            {multiSelectMode}
            isChecked={multiSelectMode && selection.has(row.id)}
            isIndeterminate={multiSelectMode &&
              selectionSummary.indeterminateIds.has(row.id)}
            onCheckClick={
              multiSelectMode ? () => handleSelectionCheckClick(row.id) : undefined
            }
            onTagTooltipChange={handleTagTooltipChange}
          />
        {/each}
        <div style={`height: ${bottomPadding}px`} aria-hidden="true"></div>
      </div>
    {/if}
  </div>

  <footer class="border-t border-border px-2 py-1 text-[11px] text-muted-foreground">
    <div class="flex items-center justify-start">
      {#if websocketStatus === WebSocketConnectionStatus.CONNECTING || websocketStatus === WebSocketConnectionStatus.RECONNECTING}
        <LoaderCircle
          class="h-3.5 w-3.5 animate-spin text-amber-500"
          aria-label="WebSocket connecting"
        />
      {:else if websocketStatus === WebSocketConnectionStatus.CONNECTED}
        <Circle
          class="h-3.5 w-3.5 fill-emerald-500 text-emerald-500"
          aria-label="WebSocket connected"
        />
      {:else}
        <Circle
          class="h-3.5 w-3.5 fill-red-500 text-red-500"
          aria-label="WebSocket disconnected"
        />
      {/if}
    </div>
  </footer>
</section>

{#if tagTooltip}
  <TagMetadataTooltip
    x={tagTooltip.x}
    y={tagTooltip.y}
    node={tagTooltip.node}
    width={TAG_TOOLTIP_WIDTH}
  />
{/if}

<VariableTreeAddDialog
  bind:open={addDialogOpen}
  connected={isConnected}
  parentId={addDialogParentId}
  {onCreateItem}
/>

<VariableTreeEditDialog
  bind:open={editDialogOpen}
  connected={isConnected}
  node={editDialogNode}
  {onEditMeta}
  onRefreshParent={(parentId) => tree.refreshNode(parentId)}
/>
