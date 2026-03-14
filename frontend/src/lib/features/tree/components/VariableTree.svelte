<script lang="ts">
  import { get } from "svelte/store";
  import { onMount } from "svelte";
  import { Circle, LoaderCircle } from "lucide-svelte";
  import TreeRow from "./TreeRow.svelte";
  import { fetchTreeChildren } from "../server-adapter";
  import { createTreeStore } from "../tree-store";
  import type { TreeNode } from "../types";
  import {
    type TagScalarValue,
    WebSocketConnectionStatus,
  } from "$lib/core/ws/types";
  import { ItemType, VarDataType } from "$lib/proto/namespace/enums";

  interface Props {
    onNodeContextMenu: (event: MouseEvent, node: TreeNode) => void;
    onNodeDragStart: (event: DragEvent, node: TreeNode) => void;
    onNodeDragEnd: (event: DragEvent) => void;
    websocketStatus?: WebSocketConnectionStatus;
    realtimeEnabled?: boolean;
    liveTagValues?: Record<string, TagScalarValue>;
    onCreateItem: (input: {
      parentId: string;
      name: string;
      itemType: ItemType;
      varType: VarDataType | undefined;
      defaultValue?: TagScalarValue;
    }) => Promise<void>;
  }

  let {
    onNodeContextMenu,
    onNodeDragStart,
    onNodeDragEnd,
    websocketStatus = WebSocketConnectionStatus.DISCONNECTED,
    realtimeEnabled = false,
    liveTagValues = {},
    onCreateItem,
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

  let treeViewportEl: HTMLDivElement | null = null;
  let addDialog: HTMLDialogElement | null = null;
  let scrollTop = $state(0);
  let viewportHeight = $state(0);
  let addName = $state("");
  let addKind = $state<ItemType>(ItemType.ITEM_TYPE_VARIABLE);
  let addDataType = $state<VarDataType>(VarDataType.VAR_DATA_TYPE_TEXT);
  let addDefaultValue = $state<string | number>("");
  let addError = $state("");
  let addSubmitting = $state(false);
  let addParentId = $state<string | null | undefined>(undefined);
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

  onMount(() => {
    void tree.initialize();

    if (!treeViewportEl) {
      return;
    }

    viewportHeight = treeViewportEl.clientHeight;
    const observer = new ResizeObserver(() => {
      if (!treeViewportEl) {
        return;
      }
      viewportHeight = treeViewportEl.clientHeight;
    });
    observer.observe(treeViewportEl);

    const refreshHandler = (event: Event) => {
      const custom = event as CustomEvent<{ parentId?: string | null }>;
      void tree.refreshNode(custom.detail?.parentId ?? null);
    };
    window.addEventListener("tree:refresh", refreshHandler as EventListener);
    const openAddDialogHandler = (event: Event) => {
      const custom = event as CustomEvent<{ parentId?: string | null }>;
      addParentId = custom.detail?.parentId;
      openAddDialog();
    };
    window.addEventListener(
      "tree:open-add-dialog",
      openAddDialogHandler as EventListener,
    );

    return () => {
      observer.disconnect();
      window.removeEventListener(
        "tree:refresh",
        refreshHandler as EventListener,
      );
      window.removeEventListener(
        "tree:open-add-dialog",
        openAddDialogHandler as EventListener,
      );
    };
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

  function openAddDialog(): void {
    if (!isConnected || !addDialog) {
      return;
    }
    addName = "";
    addKind = ItemType.ITEM_TYPE_VARIABLE;
    addDataType = VarDataType.VAR_DATA_TYPE_TEXT;
    addDefaultValue = "";
    addError = "";
    addDialog.showModal();
  }

  function closeAddDialog(): void {
    addParentId = undefined;
    if (addDialog?.open) {
      addDialog.close();
    }
  }

  function resolveDefaultInputType(dataType: VarDataType): "text" | "number" {
    return dataType === VarDataType.VAR_DATA_TYPE_INTEGER ||
      dataType === VarDataType.VAR_DATA_TYPE_FLOAT
      ? "number"
      : "text";
  }

  function resolveDefaultInputStep(dataType: VarDataType): string | undefined {
    if (dataType === VarDataType.VAR_DATA_TYPE_INTEGER) {
      return "1";
    }
    if (dataType === VarDataType.VAR_DATA_TYPE_FLOAT) {
      return "0.01";
    }
    return undefined;
  }

  function resolveDefaultInputMin(dataType: VarDataType): string | undefined {
    if (
      dataType === VarDataType.VAR_DATA_TYPE_INTEGER ||
      dataType === VarDataType.VAR_DATA_TYPE_FLOAT
    ) {
      return "0";
    }
    return undefined;
  }

  function parseDefaultValue(
    raw: string | number | null | undefined,
    dataType: VarDataType,
  ): TagScalarValue | undefined {
    const normalized = typeof raw === "number" ? String(raw) : (raw ?? "");
    if (!normalized.trim()) {
      return undefined;
    }
    if (dataType === VarDataType.VAR_DATA_TYPE_TEXT) {
      return normalized;
    }
    if (dataType === VarDataType.VAR_DATA_TYPE_BOOLEAN) {
      const booleanValue = normalized.trim().toLowerCase();
      if (booleanValue === "true") {
        return true;
      }
      if (booleanValue === "false") {
        return false;
      }
      throw new Error("Boolean default must be true or false");
    }
    const num = Number(normalized);
    if (!Number.isFinite(num)) {
      throw new Error("Default value must be numeric");
    }
    if (
      dataType === VarDataType.VAR_DATA_TYPE_INTEGER &&
      !Number.isInteger(num)
    ) {
      throw new Error("Integer default must be a whole number");
    }
    return num;
  }

  function resolveParentIdForCreate(): string | null {
    if (addParentId !== undefined) {
      if (addParentId === null) {
        const snapshot = get(treeState);
        return snapshot.rootIds[0] ?? null;
      }
      return addParentId;
    }

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

  function resolveRefreshTargetForCreate(parentId: string): string | null {
    const snapshot = get(treeState);
    if (snapshot.rootIds.includes(parentId)) {
      return parentId;
    }
    const parent = snapshot.nodes[parentId];
    return parent?.parentId ? parentId : parentId;
  }

  async function submitAddDialog(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    addError = "";
    if (!addName.trim()) {
      addError = "Name is required";
      return;
    }

    const parentId = resolveParentIdForCreate();
    if (!parentId) {
      addError = "Cannot resolve parent folder";
      return;
    }

    addSubmitting = true;
    try {
      const itemType: ItemType = addKind;
      const varType: VarDataType | undefined =
        itemType === ItemType.ITEM_TYPE_VARIABLE ? addDataType : undefined;
      const defaultValue =
        itemType === ItemType.ITEM_TYPE_VARIABLE
          ? parseDefaultValue(addDefaultValue, addDataType)
          : undefined;
      await onCreateItem({
        parentId,
        name: addName.trim(),
        itemType,
        varType,
        defaultValue,
      });
      await tree.refreshNode(resolveRefreshTargetForCreate(parentId));
      closeAddDialog();
    } catch (error) {
      addError =
        error instanceof Error ? error.message : "Failed to create item";
    } finally {
      addSubmitting = false;
    }
  }
</script>

<section
  class="flex h-full flex-col rounded-md border border-black/10 bg-(--bg-panel) dark:border-white/10"
  style="background-color: var(--bg-panel);"
>
  <header
    class="grid h-9 grid-cols-[1fr_90px_90px] items-center border-b border-black/10 px-2 text-[11px] tracking-wider text-(--text-muted) uppercase dark:border-white/10"
  >
    <span> </span>
    <span>Value</span>
    <span>Type</span>
  </header>

  <div
    class="flex-1 overflow-auto"
    bind:this={treeViewportEl}
    onscroll={() => {
      if (!treeViewportEl) {
        return;
      }
      scrollTop = treeViewportEl.scrollTop;
    }}
  >
    {#if !$treeState.hasInitialized || $treeState.rootLoading}
      <div class="space-y-2 p-2">
        {#each skeletonRows as rowKey (rowKey)}
          <div
            class="grid h-8 grid-cols-[1fr_90px_90px] items-center gap-2 rounded px-2"
          >
            <div class="h-3.5 animate-pulse rounded bg-(--bg-muted)"></div>
            <div class="h-3.5 animate-pulse rounded bg-(--bg-muted)"></div>
            <div class="h-3.5 animate-pulse rounded bg-(--bg-muted)"></div>
          </div>
        {/each}
      </div>
    {:else if $visibleRows.length === 0}
      <div class="p-3 text-xs text-(--text-muted)">No nodes available</div>
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
          />
        {/each}
        <div style={`height: ${bottomPadding}px`} aria-hidden="true"></div>
      </div>
    {/if}
  </div>

  <footer
    class="border-t border-black/10 px-2 py-1 text-[11px] text-(--text-muted) dark:border-white/10"
  >
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

<dialog
  bind:this={addDialog}
  class="fixed inset-0 m-auto h-[450px] w-[420px] rounded-md border border-black/10 bg-(--bg-panel) p-0 text-(--text-primary) shadow-xl backdrop:bg-black/50 dark:border-white/10"
>
  <form class="flex h-full flex-col p-4" onsubmit={submitAddDialog}>
    <div class="flex items-center justify-between pb-4">
      <h2 class="text-sm font-semibold">Add Node</h2>
    </div>

    <div class="space-y-4">
      <div class="space-y-1">
        <label class="text-xs text-(--text-muted)" for="add-name">Name</label>
        <input
          id="add-name"
          type="text"
          class="w-full rounded border border-black/15 bg-(--bg-muted) px-2 py-1.5 text-sm outline-none ring-0 focus:border-blue-500 dark:border-white/10"
          bind:value={addName}
          placeholder="Enter node name"
          required
        />
      </div>

      <div class="space-y-1">
        <label class="text-xs text-(--text-muted)" for="add-kind"
          >Node Type</label
        >
        <select
          id="add-kind"
          class="w-full rounded border border-black/15 bg-(--bg-muted) px-2 py-1.5 text-sm outline-none ring-0 focus:border-blue-500 dark:border-white/10"
          bind:value={addKind}
        >
          <option value={ItemType.ITEM_TYPE_FOLDER}>Folder</option>
          <option value={ItemType.ITEM_TYPE_VARIABLE}>Variable</option>
        </select>
      </div>

      <div
        class={`space-y-1 ${addKind !== ItemType.ITEM_TYPE_VARIABLE ? "invisible" : ""}`}
      >
        <label class="text-xs text-(--text-muted)" for="add-dataType"
          >Data Type</label
        >
        <select
          id="add-dataType"
          class="w-full rounded border border-black/15 bg-(--bg-muted) px-2 py-1.5 text-sm outline-none ring-0 focus:border-blue-500 dark:border-white/10"
          bind:value={addDataType}
          disabled={addKind !== ItemType.ITEM_TYPE_VARIABLE}
        >
          <option value={VarDataType.VAR_DATA_TYPE_INTEGER}>Integer</option>
          <option value={VarDataType.VAR_DATA_TYPE_FLOAT}>Float</option>
          <option value={VarDataType.VAR_DATA_TYPE_TEXT}>Text</option>
          <option value={VarDataType.VAR_DATA_TYPE_BOOLEAN}>Boolean</option>
        </select>
      </div>

      <div
        class={`space-y-1 ${addKind !== ItemType.ITEM_TYPE_VARIABLE ? "invisible" : ""}`}
      >
        <label class="text-xs text-(--text-muted)" for="add-defaultValue"
          >Default Value</label
        >
        {#if addDataType === VarDataType.VAR_DATA_TYPE_BOOLEAN}
          <select
            id="add-defaultValue"
            class="w-full rounded border border-black/15 bg-(--bg-muted) px-2 py-1.5 text-sm outline-none ring-0 focus:border-blue-500 dark:border-white/10"
            bind:value={addDefaultValue}
          >
            <option value="">Optional boolean value</option>
            <option value="true">True</option>
            <option value="false">False</option>
          </select>
        {:else}
          <input
            id="add-defaultValue"
            type={resolveDefaultInputType(addDataType)}
            class="w-full rounded border border-black/15 bg-(--bg-muted) px-2 py-1.5 text-sm outline-none ring-0 focus:border-blue-500 dark:border-white/10"
            bind:value={addDefaultValue}
            step={resolveDefaultInputStep(addDataType)}
            min={resolveDefaultInputMin(addDataType)}
            placeholder={addDataType === VarDataType.VAR_DATA_TYPE_TEXT
              ? "Optional text value"
              : "Optional numeric value"}
          />
        {/if}
      </div>
    </div>

    {#if addError}
      <p class="pt-2 text-xs text-red-500">{addError}</p>
    {/if}

    <div
      class="mt-auto flex justify-end gap-2 border-t border-black/10 pt-4 dark:border-white/10"
    >
      <button
        type="button"
        class="cursor-pointer rounded border border-black/15 px-3 py-1.5 text-xs hover:bg-(--bg-hover) dark:border-white/10"
        onclick={closeAddDialog}
      >
        Cancel
      </button>
      <button
        type="submit"
        class="cursor-pointer rounded bg-blue-600 px-3 py-1.5 text-xs text-white hover:bg-blue-500 disabled:cursor-not-allowed disabled:opacity-60"
        disabled={addSubmitting || !isConnected}
      >
        Save
      </button>
    </div>
  </form>
</dialog>
