<script lang="ts">
  import { browser } from "$app/environment";
  import { env } from "$env/dynamic/public";
  import { onDestroy, onMount } from "svelte";
  import { get, writable } from "svelte/store";
  import {
    addEdge,
    Background,
    ConnectionLineType,
    type Connection,
    Controls,
    MiniMap,
    SvelteFlow,
    type Edge,
    type Node,
    type Viewport,
  } from "@xyflow/svelte";
  import "@xyflow/svelte/dist/style.css";
  import { Button } from "$lib/components/Button";
  import VariableTree from "$lib/features/tree/components/VariableTree.svelte";
  import ContextMenu from "$lib/features/tree/components/ContextMenu.svelte";
  import PageToolbar from "$lib/features/workspace/components/PageToolbar.svelte";
  import NamespaceBuilder from "$lib/features/namespace-builder/components/NamespaceBuilder.svelte";
  import PlantAssetNode from "$lib/features/graph/components/PlantAssetNode.svelte";
  import { getRegisteredAssetDefinitions } from "$lib/features/graph/assets/registry";
  import { PlantAssetKind } from "$lib/features/graph/assets/types";
  import {
    applyLiveValuesToGraphNodes,
    getTrackedTagIds,
    normalizePipeEdges,
  } from "$lib/features/graph/live-utils";
  import { createPageTagRealtimeProvider } from "$lib/features/realtime/page-tag-realtime-provider";
  import { createThemeVars } from "$lib/core/theme/theme-utils";
  import { tagStreamClient } from "$lib/core/ws/tag-stream-client";
  import { snackbarStore } from "$lib/stores/snackbar";
  import { themeStore } from "$lib/stores/theme";
  import type { TagScalarValue } from "$lib/core/ws/types";
  import { ItemType, type VarDataType } from "$lib/proto/namespace/enums";
  import type {
    MenuContext,
    MenuOptionsResolver,
    MenuResolverByKind,
  } from "$lib/features/tree/context-menu";
  import type { TreeNode } from "$lib/features/tree/types";

  interface ActiveMenuState {
    x: number;
    y: number;
    context: MenuContext;
    resolver: MenuOptionsResolver;
  }

  interface DragPreviewState {
    x: number;
    y: number;
  }

  type CanvasMode = "edit" | "play";

  const DEMO_WS_ENDPOINT = env.PUBLIC_DEMO_WS_ENDPOINT || "ws://127.0.0.1:1236";
  const PIPE_EDGE_TYPE = "step";
  const PIPE_EDGE_STYLE = "stroke:#5b708a;stroke-width:8;";
  const theme = themeStore;

  // Apply theme to <html> only after it has been loaded from storage (or default); no theme until then.
  $: if (browser && $theme !== null) {
    const themeClass = $theme === "dark" ? "theme-dark" : "theme-light";
    const other = $theme === "dark" ? "theme-light" : "theme-dark";
    document.documentElement.classList.remove(other);
    document.documentElement.classList.add(themeClass);
  }

  const realtimeProvider = createPageTagRealtimeProvider(DEMO_WS_ENDPOINT);
  const nodeTypes = {
    plantAsset: PlantAssetNode,
  };
  const wsStatus = realtimeProvider.status;
  const tagValues = realtimeProvider.values;

  let activeMenu: ActiveMenuState | null = null;
  let draggingNode: TreeNode | null = null;
  let dragPreview: DragPreviewState | null = null;
  let canvasMode: CanvasMode = "edit";
  let graphNodes: Node[] = [];
  let graphEdges: Edge[] = [];
  let graphHostRef: HTMLElement | null = null;
  let graphNodeCounter = 0;
  let graphEdgeCounter = 0;
  let graphViewport: Viewport = { x: 0, y: 0, zoom: 1 };
  let subscribedTagIds: string[] = [];
  let removeDialog: HTMLDialogElement | null = null;
  let namespaceBuilderDialog: HTMLDialogElement | null = null;
  let namespaceBuilderRef: NamespaceBuilder | null = null;
  let namespaceBuilderValid = true;
  let namespaceBuilderCreateLoading = false;
  /** When opening from toolbar: "" and "Root"; from folder context: folder id and folder name. */
  let namespaceBuilderParentId = "";
  let namespaceBuilderParentName = "Root";
  let removeTargetNode: TreeNode | null = null;
  let removeSubmitting = false;
  let removeError = "";
  const transparentDragImage: HTMLImageElement | null = browser
    ? new Image()
    : null;
  if (transparentDragImage) {
    transparentDragImage.src =
      "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIxIiBoZWlnaHQ9IjEiLz4=";
  }

  const nodeMenuResolvers: MenuResolverByKind = {
    folder: (context) => [
      /* {
				id: 'folder-open',
				label: `Open ${context.node.name}`,
				onSelect: () => {
					console.info('Open folder', context.node.path);
				}
			}, */
      /* {
				id: 'folder-tools',
				label: 'Folder Tools',
				getChildren: async () => {
					await new Promise((resolve) => setTimeout(resolve, 120));
					return [
						{
							id: 'folder-refresh',
							label: 'Refresh Branch',
							onSelect: () => console.info('Refresh', context.node.path)
						},
						{
							id: 'folder-export',
							label: 'Export',
							children: [
								{
									id: 'folder-export-json',
									label: 'As JSON',
									onSelect: () => console.info('Export JSON', context.node.path)
								},
								{
									id: 'folder-export-csv',
									label: 'As CSV',
									onSelect: () => console.info('Export CSV', context.node.path)
								}
							]
						}
					];
				}
			}, */
      {
        id: "folder-add",
        label: "Add",
        onSelect: () => {
          window.dispatchEvent(
            new CustomEvent<{ parentId?: string | null }>(
              "tree:open-add-dialog",
              {
                detail: { parentId: context.node.id },
              },
            ),
          );
        },
      },
      ...(context.node.parentId
        ? [
            {
              id: "folder-remove",
              label: "Remove",
              disabled: get(wsStatus) !== "connected",
              onSelect: () => {
                openRemoveDialog(context.node);
              },
            },
          ]
        : []),
      { id: "folder-sep-ns", label: "", separator: true },
      {
        id: "folder-namespace-builder",
        label: "Namespace Template Builder",
        onSelect: () => {
          openNamespaceBuilderDialog(context.node.id, context.node.name);
        },
      },
    ],
    tag: (context) => [
      /* {
				id: 'tag-inspect',
				label: `Inspect ${context.node.name}`,
				onSelect: () => console.info('Inspect tag', context.node.path)
			}, */
      /* {
				id: 'tag-history',
				label: 'Trend & History',
				children: [
					{
						id: 'tag-history-1h',
						label: 'Last 1 hour',
						onSelect: () => console.info('History 1h', context.node.path)
					},
					{
						id: 'tag-history-24h',
						label: 'Last 24 hours',
						onSelect: () => console.info('History 24h', context.node.path)
					}
				]
			}, */
      {
        id: "tag-remove",
        label: "Remove",
        disabled: get(wsStatus) !== "connected",
        onSelect: () => {
          openRemoveDialog(context.node);
        },
      },
    ],
  };

  function buildDropAssetMenuOptions(context: MenuContext) {
    return getRegisteredAssetDefinitions().map((definition) => ({
      id: definition.name,
      label: definition.label,
      onSelect: () =>
        onDropAction(context.event, context.node, definition.name),
    }));
  }

  const dropMenuResolvers: MenuResolverByKind = {
    folder: (context) => [
      {
        id: "drop-folder-assets",
        label: "Add Plant Asset",
        children: buildDropAssetMenuOptions(context),
      },
    ],
    tag: (context) => [
      {
        id: "drop-tag-assets",
        label: "Add Plant Asset",
        children: buildDropAssetMenuOptions(context),
      },
    ],
  };

  async function createTreeItem(input: {
    parentId: string;
    name: string;
    itemType: ItemType;
    varType: VarDataType | undefined;
  }): Promise<void> {
    await realtimeProvider.addItem(
      input.parentId,
      input.name,
      input.itemType,
      input.varType,
    );
  }

  async function removeTreeNode(node: TreeNode): Promise<void> {
    await realtimeProvider.removeItems([node.id]);
  }

  function openRemoveDialog(node: TreeNode): void {
    if (get(wsStatus) !== "connected" || !removeDialog) {
      return;
    }
    removeTargetNode = node;
    removeSubmitting = false;
    removeError = "";
    removeDialog.showModal();
  }

  function closeRemoveDialog(force = false): void {
    if (removeSubmitting && !force) {
      return;
    }
    if (removeDialog?.open) {
      removeDialog.close();
    }
    removeTargetNode = null;
    removeError = "";
  }

  async function confirmRemoveTargetNode(): Promise<void> {
    if (!removeTargetNode) {
      return;
    }
    if (get(wsStatus) !== "connected") {
      removeError =
        "WebSocket is disconnected. Please reconnect and try again.";
      return;
    }

    removeSubmitting = true;
    removeError = "";
    try {
      await removeTreeNode(removeTargetNode);
      closeRemoveDialog(true);
    } catch (error) {
      removeError =
        error instanceof Error ? error.message : "Failed to remove node";
    } finally {
      removeSubmitting = false;
    }
  }

  function openContextMenu(
    event: MouseEvent,
    context: MenuContext,
    resolverByKind: MenuResolverByKind,
  ): void {
    activeMenu = {
      x: event.clientX,
      y: event.clientY,
      context,
      resolver: resolverByKind[context.node.kind],
    };
  }

  function handleNodeContextMenu(event: MouseEvent, node: TreeNode): void {
    openContextMenu(event, { node, event, kind: "node" }, nodeMenuResolvers);
  }

  function handleNodeDragStart(event: DragEvent, node: TreeNode): void {
    if (canvasMode === "play") {
      event.preventDefault();
      return;
    }

    draggingNode = node;
    dragPreview = {
      x: event.clientX + 12,
      y: event.clientY + 12,
    };
    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = "copy";
      event.dataTransfer.setData("text/plain", node.id);
      if (transparentDragImage) {
        event.dataTransfer.setDragImage(transparentDragImage, 0, 0);
      }
    }
  }

  function handleNodeDragEnd(): void {
    draggingNode = null;
    dragPreview = null;
  }

  function updateDragPreview(event: DragEvent): void {
    if (!draggingNode) {
      return;
    }

    dragPreview = {
      x: event.clientX + 12,
      y: event.clientY + 12,
    };
  }

  function handleRightPanelDragOver(event: DragEvent): void {
    if (canvasMode === "play") {
      return;
    }

    event.preventDefault();
    updateDragPreview(event);
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = "copy";
    }
  }

  function handleRightPanelDrop(event: DragEvent): void {
    if (canvasMode === "play") {
      return;
    }

    event.preventDefault();
    if (!draggingNode) {
      return;
    }

    openContextMenu(
      event,
      { node: draggingNode, event, kind: "drop" },
      dropMenuResolvers,
    );
  }

  function getGraphPositionFromEvent(event: MouseEvent): {
    x: number;
    y: number;
  } {
    if (!graphHostRef) {
      return { x: event.clientX, y: event.clientY };
    }

    const rect = graphHostRef.getBoundingClientRect();
    const localX = event.clientX - rect.left;
    const localY = event.clientY - rect.top;
    const flowX = (localX - graphViewport.x) / graphViewport.zoom;
    const flowY = (localY - graphViewport.y) / graphViewport.zoom;

    return {
      x: flowX - 95,
      y: flowY - 45,
    };
  }

  function handleFlowMove(
    _event: MouseEvent | TouchEvent | null,
    viewport: Viewport,
  ): void {
    graphViewport = viewport;
  }

  function onDropAction(
    event: MouseEvent,
    node: TreeNode,
    assetKind: PlantAssetKind,
  ): void {
    const position = getGraphPositionFromEvent(event);
    graphNodeCounter += 1;

    const newNode: Node = {
      id: `asset-${assetKind}-${graphNodeCounter}`,
      type: "plantAsset",
      position,
      data: {
        symbolId: `asset-${assetKind}-${graphNodeCounter}`,
        assetKind,
        title: `${assetKind.toUpperCase()} ${graphNodeCounter}`,
        sourceNode: {
          id: node.id,
          name: node.name,
          path: node.path,
          kind: node.kind,
          dataType: node.dataType,
        },
        onWriteValue:
          assetKind === PlantAssetKind.SLIDER ||
          assetKind === PlantAssetKind.ONOFF ||
          assetKind === PlantAssetKind.TYPED_INPUT
            ? (value: TagScalarValue) =>
                realtimeProvider.sendWriteValue(node.id, value)
            : undefined,
      },
    };

    graphNodes = [...graphNodes, newNode];
  }

  function handleConnect(connection: Connection): void {
    if (canvasMode !== "edit") {
      return;
    }

    graphEdgeCounter += 1;
    graphEdges = addEdge(
      {
        ...connection,
        id: `pipe-${graphEdgeCounter}`,
        type: PIPE_EDGE_TYPE,
        animated: false,
        style: PIPE_EDGE_STYLE,
      },
      graphEdges,
    );
  }

  function toggleCanvasMode(): void {
    canvasMode = canvasMode === "edit" ? "play" : "edit";
  }

  function toggleTheme(): void {
    themeStore.update((current) => (current === "dark" ? "light" : "dark"));
  }

  /** Opens the Add Variable/Folder dialog (same as tree context menu) at root. */
  function openTreeAddDialog(): void {
    window.dispatchEvent(
      new CustomEvent<{ parentId?: string | null }>("tree:open-add-dialog", {
        detail: { parentId: null },
      }),
    );
  }

  /** Opens the Namespace Template Builder at root (used by toolbar). */
  function openNamespaceBuilderFromToolbar(): void {
    openNamespaceBuilderDialog("", "root");
  }

  /** Opens the Namespace Template Builder dialog (bulk add from YAML). parentId: "" for root, or folder id. parentName: "Root" or folder name for dialog title. */
  function openNamespaceBuilderDialog(
    parentId: string,
    parentName: string,
  ): void {
    namespaceBuilderParentId = parentId;
    namespaceBuilderParentName = parentName;
    namespaceBuilderDialog?.showModal();
    if (
      namespaceBuilderRef &&
      typeof namespaceBuilderRef.getValidity === "function"
    ) {
      namespaceBuilderValid = namespaceBuilderRef.getValidity();
    }
  }

  function closeNamespaceBuilderDialog(): void {
    if (
      namespaceBuilderRef &&
      typeof namespaceBuilderRef.reset === "function"
    ) {
      namespaceBuilderRef.reset();
    }
    if (namespaceBuilderDialog?.open) {
      namespaceBuilderDialog.close();
    }
  }

  async function onNamespaceBuilderCreate(): Promise<void> {
    if (
      !namespaceBuilderRef ||
      typeof namespaceBuilderRef.buildNamespaceJsonFromYaml !== "function" ||
      namespaceBuilderCreateLoading
    ) {
      return;
    }
    let json: Record<string, unknown>;
    try {
      json = namespaceBuilderRef.buildNamespaceJsonFromYaml() as Record<
        string,
        unknown
      >;
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      snackbarStore.error(`Invalid namespace YAML: ${msg}`);
      return;
    }
    namespaceBuilderCreateLoading = true;
    try {
      await tagStreamClient.addBulkNamespace(
        namespaceBuilderParentId,
        json,
        DEMO_WS_ENDPOINT,
      );
      if (browser) {
        (
          window as unknown as { __lastNamespaceJson?: unknown }
        ).__lastNamespaceJson = json;
      }
      closeNamespaceBuilderDialog();
    } catch {
      /* Error already shown via snackbar; re-enable actions immediately so user can retry without waiting for snackbar to dismiss */
      namespaceBuilderCreateLoading = false;
    } finally {
      namespaceBuilderCreateLoading = false;
    }
  }

  function isTypingTarget(target: EventTarget | null): boolean {
    if (!(target instanceof HTMLElement)) {
      return false;
    }

    return (
      target.tagName === "INPUT" ||
      target.tagName === "TEXTAREA" ||
      target.tagName === "SELECT" ||
      target.isContentEditable
    );
  }

  function deleteSelectedGraphNodes(): void {
    const selectedIds = graphNodes
      .filter((node) => Boolean(node.selected))
      .map((node) => node.id);
    const selectedEdgeIds = graphEdges
      .filter((edge) => Boolean(edge.selected))
      .map((edge) => edge.id);

    if (selectedIds.length === 0 && selectedEdgeIds.length === 0) {
      return;
    }

    graphNodes = graphNodes.filter((node) => !selectedIds.includes(node.id));
    graphEdges = graphEdges.filter(
      (edge) =>
        !selectedIds.includes(edge.source) &&
        !selectedIds.includes(edge.target) &&
        !selectedEdgeIds.includes(edge.id),
    );
  }

  function closeMenu(): void {
    activeMenu = null;
  }

  onMount(() => {
    realtimeProvider.start();

    const handleDeleteKey = (event: KeyboardEvent) => {
      if (event.key !== "Delete") {
        return;
      }
      if (canvasMode !== "edit" || isTypingTarget(event.target)) {
        return;
      }

      event.preventDefault();
      deleteSelectedGraphNodes();
    };

    window.addEventListener("keydown", handleDeleteKey);
    return () => {
      window.removeEventListener("keydown", handleDeleteKey);
    };
  });

  onDestroy(() => {
    realtimeProvider.stop();
  });

  $: realtimeProvider.setActive(canvasMode === "play");
  $: subscribedTagIds = getTrackedTagIds(graphNodes);
  $: realtimeProvider.setDesiredIds(subscribedTagIds);
  $: if (canvasMode === "play") {
    const result = applyLiveValuesToGraphNodes(graphNodes, $tagValues);
    if (result.changed) {
      graphNodes = result.nodes;
    }
  }
  $: {
    const result = normalizePipeEdges(
      graphEdges,
      PIPE_EDGE_TYPE,
      PIPE_EDGE_STYLE,
    );
    if (result.changed) {
      graphEdges = result.edges;
    }
  }

  $: themeVars = $theme !== null ? createThemeVars($theme) : "";
</script>

<main
  class={`h-screen w-full p-4 ${$theme !== null ? ($theme === "dark" ? "theme-dark" : "theme-light") : ""} bg-(--bg-app)`}
  style={`background-color: var(--bg-app); color: var(--text-primary); ${themeVars}`}
>
  <PageToolbar
    theme={$theme ?? "light"}
    {canvasMode}
    onToggleCanvasMode={toggleCanvasMode}
    onToggleTheme={toggleTheme}
    onOpenAddDialog={openTreeAddDialog}
    onOpenNamespaceBuilder={openNamespaceBuilderFromToolbar}
    isAddDisabled={false}
  />

  <div class="flex h-[calc(100vh-5rem)] gap-4">
    <section class="h-full w-[30%] min-w-[360px]">
      <VariableTree
        onNodeContextMenu={handleNodeContextMenu}
        onNodeDragStart={handleNodeDragStart}
        onNodeDragEnd={handleNodeDragEnd}
        onCreateItem={createTreeItem}
        websocketStatus={$wsStatus}
        realtimeEnabled={canvasMode === "play"}
        liveTagValues={$tagValues}
      />
    </section>

    <section
      class="relative h-full flex-1 rounded-md border border-black/10 bg-(--bg-panel) p-0 text-sm text-(--text-secondary) dark:border-white/10"
      style="background-color: var(--bg-panel);"
      aria-label="Drop workspace"
      ondragover={handleRightPanelDragOver}
      ondrop={handleRightPanelDrop}
    >
      <div bind:this={graphHostRef} class="h-full w-full">
        <SvelteFlow
          bind:nodes={graphNodes}
          bind:edges={graphEdges}
          {nodeTypes}
          initialViewport={{ x: 0, y: 0, zoom: 1 }}
          minZoom={0.4}
          maxZoom={1.6}
          zoomOnDoubleClick={false}
          colorMode={$theme ?? "light"}
          class="h-full w-full rounded-md"
          style="background-color: var(--bg-muted);"
          nodesDraggable={canvasMode === "edit"}
          elementsSelectable={canvasMode === "edit"}
          nodesConnectable={canvasMode === "edit"}
          selectionOnDrag={canvasMode === "edit"}
          panOnDrag={[1]}
          connectionLineType={ConnectionLineType.Step}
          connectionLineStyle={PIPE_EDGE_STYLE}
          proOptions={{ hideAttribution: true }}
          onmove={handleFlowMove}
          onconnect={handleConnect}
        >
          <Controls />
          <MiniMap />
          <Background />
        </SvelteFlow>
      </div>
      <!-- <div class="pointer-events-none absolute left-3 top-2 text-[11px] text-(--text-muted)">
				{#if canvasMode === 'edit'}
					Edit mode: drop tags and place assets. No polling is active.
				{:else}
					Play mode: 2s polling active for all tag IDs on canvas.
				{/if}
			</div> -->
    </section>
  </div>

  {#if dragPreview && draggingNode}
    <div
      class="pointer-events-none fixed z-40 rounded border border-black/10 bg-(--bg-panel) px-2 py-1 text-xs shadow-md dark:border-white/10"
      style={`background-color: var(--bg-panel); left:${dragPreview.x}px;top:${dragPreview.y}px;`}
    >
      Dragging: {draggingNode.name}
    </div>
  {/if}

  {#if activeMenu}
    <ContextMenu
      anchorX={activeMenu.x}
      anchorY={activeMenu.y}
      context={activeMenu.context}
      rootResolver={activeMenu.resolver}
      onClose={closeMenu}
    />
  {/if}

  <dialog
    bind:this={removeDialog}
    class="fixed inset-0 m-auto w-[420px] rounded-md border border-black/10 bg-(--bg-panel) p-0 text-(--text-primary) shadow-xl backdrop:bg-black/50 dark:border-white/10"
  >
    <form
      class="flex flex-col p-4"
      onsubmit={(event) => {
        event.preventDefault();
        void confirmRemoveTargetNode();
      }}
    >
      <div class="space-y-2">
        <h2 class="text-sm font-semibold">Confirm removal</h2>
        <p class="text-xs text-(--text-muted)">
          Remove "{removeTargetNode?.name}" ({removeTargetNode?.kind ===
          "folder"
            ? "folder"
            : "variable"})? This action cannot be undone.
        </p>
        {#if removeError}
          <p class="text-xs text-red-500">{removeError}</p>
        {/if}
      </div>
      <div
        class="mt-4 flex justify-end gap-2 border-t border-black/10 pt-4 dark:border-white/10"
      >
        <Button
          variant="outline-muted"
          label="Cancel"
          title="Cancel"
          disabled={removeSubmitting}
          onclick={() => closeRemoveDialog()}
        />
        <Button
          type="submit"
          variant="filled-warn"
          label="Remove"
          loadingLabel="Removing..."
          loading={removeSubmitting}
          disabled={removeSubmitting ||
            $wsStatus !== "connected" ||
            !removeTargetNode}
        />
      </div>
    </form>
  </dialog>

  <dialog
    bind:this={namespaceBuilderDialog}
    class="fixed inset-0 m-auto h-[82vh] w-[92vw] max-w-[1400px] rounded-md border border-black/10 bg-(--bg-panel) p-0 text-(--text-primary) shadow-xl backdrop:bg-black/60 dark:border-white/10"
  >
    <div class="flex h-full flex-col p-3">
      <div class="mb-2 flex items-center justify-between">
        <h2 class="text-sm font-semibold">
          Namespace Template Builder — {namespaceBuilderParentName}
        </h2>
      </div>
      <div class="min-h-0 flex-1">
        <NamespaceBuilder
          bind:this={namespaceBuilderRef}
          colorMode={$theme ?? "light"}
          createLoading={namespaceBuilderCreateLoading}
          onValidityChange={(v) => (namespaceBuilderValid = v)}
        />
      </div>
      <div
        class="mt-2 flex justify-end gap-2 border-t border-black/10 pt-3 dark:border-white/10"
      >
        {#if !namespaceBuilderCreateLoading}
          <Button
            variant="outline-muted"
            label="Cancel"
            title="Cancel"
            onclick={closeNamespaceBuilderDialog}
          />
        {/if}
        <Button
          variant="filled-accent"
          label="Create"
          title="Create"
          loading={namespaceBuilderCreateLoading}
          loadingLabel="Creating…"
          disabled={!namespaceBuilderValid || namespaceBuilderCreateLoading}
          onclick={() => void onNamespaceBuilderCreate()}
        />
      </div>
    </div>
  </dialog>
</main>
