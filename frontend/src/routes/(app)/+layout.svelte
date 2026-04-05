<script lang="ts">
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { browser } from "$app/environment";
  import { resolveTagStreamWsEndpoint } from "$lib/core/ws/resolve-ws-endpoint";
  import { onDestroy, onMount } from "svelte";
  import { get } from "svelte/store";
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
  import * as Dialog from "$lib/components/ui/dialog";
  import { Input } from "$lib/components/ui/input";
  import VariableTree from "$lib/features/tree/components/VariableTree.svelte";
  import ContextMenu from "$lib/features/tree/components/ContextMenu.svelte";
  import PageToolbar from "$lib/features/workspace/components/PageToolbar.svelte";
  import NamespaceBuilder from "$lib/features/namespace-builder/components/NamespaceBuilder.svelte";
  import ViewsListPanel from "$lib/features/views/components/ViewsListPanel.svelte";
  import ViewEditorHeader from "$lib/features/views/components/ViewEditorHeader.svelte";
  import PlantAssetNode from "$lib/features/graph/components/PlantAssetNode.svelte";
  import {
    getRegisteredAssetDefinitions,
    resolveAssetDefinition,
  } from "$lib/features/graph/assets/registry";
  import {
    type BoundWidgetTag,
    type PlantAssetNodeData,
    type WidgetBindingSchema,
  } from "$lib/features/graph/assets/types";
  import {
    applyLiveValuesToGraphNodes,
    getTrackedTagIds,
    normalizePipeEdges,
  } from "$lib/features/graph/live-utils";
  import { createPageTagRealtimeProvider } from "$lib/features/realtime/page-tag-realtime-provider";
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
  import {
    getLoadedDescendantIds,
    getMinimalAncestorSet,
  } from "$lib/features/tree/tree-selection";
  import { CheckSquare, Layers, ListChecks, Plus, Trash2, Pencil } from "lucide-svelte";
  import { sanitizeIdentifierLike, sanitizeText } from "$lib/forms/sanitize";
  import {
    createView,
    deleteView,
    getView,
    listViews,
    setEntryPointView,
    updateView,
  } from "$lib/features/views/api/views-api";
  import {
    deserializeCanvasState,
    serializeCanvasState,
    type CanvasMode,
    type ScadaView,
  } from "$lib/features/views/types";
  import {
    DEFAULT_VIEWS_TABLE_STATE,
    type ViewsTableState,
  } from "$lib/features/views/table/views-table-state";

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

  type WorkspaceMode = "designer" | "runtime";

  const WS_ENDPOINT = resolveTagStreamWsEndpoint();
  const PIPE_EDGE_TYPE = "step";
  const PIPE_EDGE_STYLE = "stroke:#5b708a;stroke-width:8;";
  const theme = themeStore;
  const username = "Admin";

  const realtimeProvider = createPageTagRealtimeProvider(WS_ENDPOINT);
  const nodeTypes = {
    plantAsset: PlantAssetNode,
  };
  const wsStatus = realtimeProvider.status;
  const tagValues = realtimeProvider.values;

  let activeMenu = $state<ActiveMenuState | null>(null);
  let draggingNode = $state<TreeNode | null>(null);
  let dragPreview = $state<DragPreviewState | null>(null);
  let workspaceMode = $state<WorkspaceMode>("designer");
  const routeViewId = $derived($page.params.id ?? null);
  const rightPaneMode = $derived(
    routeViewId ? "view-editor" : "views-list",
  );
  let previousRightPaneMode = $state<"views-list" | "view-editor" | null>(null);
  let canvasMode = $state<CanvasMode>("edit");
  let views = $state<ScadaView[]>([]);
  let viewsTotal = $state(0);
  let viewsLoadError = $state("");
  let viewsTableState = $state<ViewsTableState>({ ...DEFAULT_VIEWS_TABLE_STATE });
  let viewsSearchInput = $state("");
  let viewsSearchQuery = $state("");
  let viewsSearchDebounceTimer = $state<ReturnType<typeof setTimeout> | null>(null);
  let viewsLoading = $state(false);
  let viewCreateLoading = $state(false);
  let viewActionBusyId = $state<string | null>(null);
  let loadedViewId = $state<string | null>(null);
  let viewEditorLoading = $state(false);
  let viewEditorError = $state("");
  let viewSaveLoading = $state(false);
  let removeViewDialogOpen = $state(false);
  let removeViewTarget = $state<ScadaView | null>(null);
  let removeViewSubmitting = $state(false);
  let removeViewError = $state("");
  // Large graph collections: keep as raw state to avoid deep proxy overhead.
  let graphNodes = $state.raw<Node[]>([]);
  let graphEdges = $state.raw<Edge[]>([]);
  let graphHostRef = $state<HTMLElement | null>(null);
  let graphNodeCounter = 0;
  let graphEdgeCounter = 0;
  let graphViewport: Viewport = { x: 0, y: 0, zoom: 1 };
  let removeDialogOpen = $state(false);
  let namespaceBuilderDialogOpen = $state(false);
  let namespaceBuilderRef: NamespaceBuilder | null = null;
  let namespaceBuilderValid = $state(true);
  let namespaceBuilderCreateLoading = $state(false);
  /** When opening from toolbar: root node id (or "" if tree not loaded); from folder context: folder id. */
  let namespaceBuilderParentId = "";
  let namespaceBuilderParentName = $state("Root");
  /** Root folder id from the variable tree (set when tree has loaded). Used as parentId when opening namespace builder from toolbar. */
  let treeRootId: string | null = null;
  let removeTargetNode = $state<TreeNode | null>(null);
  let removeSubmitting = $state(false);
  let removeError = $state("");

  /** Multi-selection mode: show checkboxes and use global selection set instead of single select. */
  let multiSelectMode = $state(false);
  /** Set of node ids selected in multi-selection mode. */
  let treeSelection = $state(new Set<string>());
  /** Snapshot of tree nodes from VariableTree for computing minimal delete set. */
  let treeNodes = $state<Record<string, TreeNode>>({});
  let treeRootIds = $state<string[]>([]);
  let removeMultipleDialogOpen = $state(false);
  let removeMultipleSubmitting = $state(false);
  let removeMultipleError = $state("");
  let inspectorDockVisible = $state(true);
  const transparentDragImage: HTMLImageElement | null = browser
    ? new Image()
    : null;
  const canDropToCanvas = $derived(
    rightPaneMode === "view-editor" && canvasMode === "edit",
  );
  const activeView = $derived(views.find((view) => view.id === routeViewId) ?? null);
  const treeActionsDisabled = $derived($wsStatus !== "connected");
  const selectionCount = $derived(treeSelection.size);
  let { children } = $props();
  if (transparentDragImage) {
    transparentDragImage.src =
      "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSIxIiBoZWlnaHQ9IjEiLz4=";
  }

  const nodeMenuResolvers: MenuResolverByKind = {
    folder: (context) => [
      {
        id: "folder-add",
        label: "Add",
        icon: Plus,
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
      {
        id: "folder-remove",
        label: "Remove",
        icon: Trash2,
        disabled: get(wsStatus) !== "connected",
        onSelect: () => {
          openRemoveDialog(context.node);
        },
      },
      { id: "folder-sep-ns", label: "", separator: true },
      {
        id: "folder-namespace-builder",
        label: "Namespace Template Builder",
        icon: Layers,
        onSelect: () => {
          openNamespaceBuilderDialog(
            context.node.id,
            formatNamespaceBuilderPath(context.node.path),
          );
        },
      },
    ],
    tag: (context) => [
      {
        id: "tag-edit",
        label: "Edit",
        icon: Pencil,
        disabled: get(wsStatus) !== "connected",
        onSelect: () => {
          window.dispatchEvent(
            new CustomEvent<{ node: TreeNode }>("tree:open-edit-dialog", {
              detail: { node: context.node },
            }),
          );
        },
      },
      {
        id: "tag-remove",
        label: "Remove",
        icon: Trash2,
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
    folder: (context) => buildDropAssetMenuOptions(context),
    tag: (context) => buildDropAssetMenuOptions(context),
  };

  function extractTrailingCounter(id: string): number {
    const match = id.match(/-(\d+)$/);
    return match ? Number(match[1]) : 0;
  }

  function syncGraphCounters(nodes: Node[], edges: Edge[]): void {
    graphNodeCounter = nodes.reduce(
      (max, node) => Math.max(max, extractTrailingCounter(node.id)),
      0,
    );
    graphEdgeCounter = edges.reduce(
      (max, edge) => Math.max(max, extractTrailingCounter(edge.id)),
      0,
    );
  }

  function hydratePlantAssetNodeData(
    nodeId: string,
    raw: PlantAssetNodeData,
  ): PlantAssetNodeData {
    const definition = resolveAssetDefinition(raw.assetKind);
    const primaryBindingKey = raw.primaryBindingKey ?? definition.primaryBindingKey;
    const canWritePrimary =
      definition.bindings.find((binding) => binding.key === primaryBindingKey)
        ?.access !== "read";

    return {
      ...raw,
      primaryBindingKey,
      onWriteValue: canWritePrimary
        ? (value: TagScalarValue) =>
            writeWidgetBindingValue(nodeId, primaryBindingKey, value)
        : undefined,
      onWriteBindingValue: (
        bindingKey: string,
        value: TagScalarValue,
        tagId?: string,
      ) => writeWidgetBindingValue(nodeId, bindingKey, value, tagId),
      onOpenBindingConfig: () => focusGraphNodeInInspector(nodeId),
    };
  }

  function hydrateCanvasNodes(nodes: Node[]): Node[] {
    return nodes.map((node) => {
      if (node.type !== "plantAsset") {
        return node;
      }

      const data = node.data as PlantAssetNodeData;
      return {
        ...node,
        data: hydratePlantAssetNodeData(node.id, data),
      };
    });
  }

  function replaceViewInState(nextView: ScadaView): void {
    if (views.some((view) => view.id === nextView.id)) {
      views = views.map((view) => (view.id === nextView.id ? nextView : view));
      return;
    }
    views = [...views, nextView];
  }

  async function loadViewsList(tableState = viewsTableState): Promise<void> {
    viewsLoading = true;
    viewsLoadError = "";
    try {
      const result = await listViews({
        page: tableState.pageIndex + 1,
        pageSize: tableState.pageSize,
        sortBy: tableState.sortBy,
        sortDirection: tableState.sortDirection,
        search: viewsSearchQuery,
      });
      views = result.items;
      viewsTotal = result.total;
    } catch (error) {
      const message =
        error instanceof Error && error.message
          ? error.message
          : "Unable to load views.";
      views = [];
      viewsTotal = 0;
      viewsLoadError = message;
      snackbarStore.error(
        "Failed to fetch views",
      );
    } finally {
      viewsLoading = false;
    }
  }

  function onViewsTableStateChange(nextState: ViewsTableState): void {
    viewsTableState = nextState;
    void loadViewsList(nextState);
  }

  function onViewsSearchChange(value: string): void {
    viewsSearchInput = value;
    if (viewsSearchDebounceTimer) {
      clearTimeout(viewsSearchDebounceTimer);
    }
    viewsSearchDebounceTimer = setTimeout(() => {
      viewsSearchDebounceTimer = null;
      const nextQuery = value.trim();
      if (viewsSearchQuery === nextQuery) {
        return;
      }
      viewsSearchQuery = nextQuery;
      const nextState = { ...viewsTableState, pageIndex: 0 };
      viewsTableState = nextState;
      void loadViewsList(nextState);
    }, 250);
  }

  async function loadViewEditor(viewId: string): Promise<void> {
    viewEditorLoading = true;
    viewEditorError = "";
    viewActionBusyId = viewId;
    try {
      const view = await getView(viewId);
      const canvas = deserializeCanvasState(view.canvas_json);
      graphNodes = hydrateCanvasNodes(canvas.nodes);
      graphEdges = canvas.edges;
      graphViewport = canvas.viewport;
      syncGraphCounters(graphNodes, graphEdges);
      replaceViewInState(view);
      canvasMode = "edit";
      loadedViewId = view.id;
    } catch (error) {
      graphNodes = [];
      graphEdges = [];
      graphViewport = { x: 0, y: 0, zoom: 1 };
      loadedViewId = null;
      viewEditorError =
        error instanceof Error && error.message
          ? error.message
          : "Unable to load this view.";
      snackbarStore.error(
        "Failed to open view",
      );
    } finally {
      viewEditorLoading = false;
      viewActionBusyId = null;
    }
  }

  function randomViewName(): string {
    const suffix = (browser && "randomUUID" in crypto
      ? crypto.randomUUID()
      : Math.random().toString(36).slice(2, 10)
    ).slice(0, 8);
    return `View ${suffix}`;
  }

  async function createViewInPlace(): Promise<void> {
    if (viewCreateLoading) {
      return;
    }
    viewCreateLoading = true;
    try {
      await createView({
        name: randomViewName(),
        description: "",
        canvas_json: serializeCanvasState([], [], { x: 0, y: 0, zoom: 1 }),
      });
      const nextState = { ...viewsTableState, pageIndex: 0 };
      viewsTableState = nextState;
      await loadViewsList(nextState);
    } catch (error) {
      snackbarStore.error(
        "Failed to create view",
      );
    } finally {
      viewCreateLoading = false;
    }
  }

  function openRemoveViewDialog(view: ScadaView): void {
    removeViewTarget = view;
    removeViewSubmitting = false;
    removeViewError = "";
    removeViewDialogOpen = true;
  }

  function closeRemoveViewDialog(force = false): void {
    if (removeViewSubmitting && !force) {
      return;
    }
    removeViewDialogOpen = false;
    removeViewTarget = null;
    removeViewError = "";
  }

  async function confirmRemoveView(): Promise<void> {
    if (!removeViewTarget) {
      return;
    }

    const view = removeViewTarget;
    removeViewSubmitting = true;
    removeViewError = "";
    viewActionBusyId = view.id;
    try {
      await deleteView(view.id);
      views = views.filter((item) => item.id !== view.id);
      if (routeViewId === view.id) {
        graphNodes = [];
        graphEdges = [];
        graphViewport = { x: 0, y: 0, zoom: 1 };
        loadedViewId = null;
        await goto("/views");
      }
      const nextTotal = Math.max(0, viewsTotal - 1);
      const pageCountAfterDelete = Math.max(
        1,
        Math.ceil(nextTotal / viewsTableState.pageSize),
      );
      const nextState = {
        ...viewsTableState,
        pageIndex: Math.min(viewsTableState.pageIndex, pageCountAfterDelete - 1),
      };
      viewsTableState = nextState;
      await loadViewsList(nextState);
      closeRemoveViewDialog(true);
    } catch (error) {
      removeViewError =
        error instanceof Error ? error.message : "Failed to remove view";
    } finally {
      removeViewSubmitting = false;
      viewActionBusyId = null;
    }
  }

  async function saveActiveViewCanvas(): Promise<void> {
    if (!activeView) {
      return;
    }

    viewSaveLoading = true;
    try {
      const updated = await updateView(activeView.id, {
        name: activeView.name,
        description: activeView.description,
        is_entry_point: activeView.is_entry_point,
        canvas_json: serializeCanvasState(graphNodes, graphEdges, graphViewport),
      });
      replaceViewInState(updated);
      snackbarStore.success("View saved.");
    } catch (error) {
      snackbarStore.error(
        "Failed to save view",
      );
    } finally {
      viewSaveLoading = false;
    }
  }

  async function setActiveViewAsEntryPoint(): Promise<void> {
    if (!activeView) return;
    viewActionBusyId = activeView.id;
    try {
      const updated = await setEntryPointView(activeView.id);
      views = views.map((view) => ({
        ...view,
        is_entry_point: view.id === updated.id,
        updated_at: view.id === updated.id ? updated.updated_at : view.updated_at,
      }));
      snackbarStore.success("Entry-point view updated.");
    } catch (error) {
      snackbarStore.error(
        "Failed to update entry-point",
      );
    } finally {
      viewActionBusyId = null;
    }
  }

  async function updateInlineViewFields(
    view: ScadaView,
    changes: { name?: string; description?: string },
  ): Promise<boolean> {
    viewActionBusyId = view.id;
    try {
      const updated = await updateView(view.id, {
        name: changes.name ?? view.name,
        description: changes.description ?? view.description,
        is_entry_point: view.is_entry_point,
        canvas_json: view.canvas_json,
      });
      replaceViewInState(updated);
      return true;
    } catch (error) {
      snackbarStore.error("Failed to update view");
      return false;
    } finally {
      viewActionBusyId = null;
    }
  }

  async function createTreeItem(input: {
    parentId: string | null;
    name: string;
    itemType: ItemType;
    varType: VarDataType | undefined;
    unit?: string;
    min?: number;
    max?: number;
    options?: string[];
    maxLen?: number;
  }): Promise<void> {
    await realtimeProvider.addItem(
      input.parentId,
      sanitizeIdentifierLike(input.name, 128),
      input.itemType,
      input.varType,
      {
        unit: input.unit ? sanitizeIdentifierLike(input.unit, 32) : undefined,
        min: input.min,
        max: input.max,
        options: input.options?.map((opt) => sanitizeIdentifierLike(opt, 64)),
        maxLen: input.maxLen,
      },
    );
  }

  async function editTreeMeta(input: {
    varId: string;
    unit?: string;
    min?: number;
    max?: number;
    options?: string[];
    maxLen?: number;
  }): Promise<void> {
    await realtimeProvider.updateMeta(input.varId, {
      unit: input.unit ? sanitizeIdentifierLike(input.unit, 32) : undefined,
      min: input.min,
      max: input.max,
      options: input.options?.map((opt) => sanitizeIdentifierLike(opt, 64)),
      maxLen: input.maxLen,
    });
  }

  async function removeTreeNode(node: TreeNode): Promise<void> {
    await realtimeProvider.removeItems([node.id]);
  }

  function openRemoveDialog(node: TreeNode): void {
    if (get(wsStatus) !== "connected") {
      return;
    }
    removeTargetNode = node;
    removeSubmitting = false;
    removeError = "";
    removeDialogOpen = true;
  }

  function closeRemoveDialog(force = false): void {
    if (removeSubmitting && !force) {
      return;
    }
    removeDialogOpen = false;
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

  function applySelectionChange(payload: {
    add: string[];
    remove: string[];
  }): void {
    const next = new Set(treeSelection);
    for (const id of payload.remove) next.delete(id);
    for (const id of payload.add) next.add(id);
    treeSelection = next;
  }

  function selectAllSelection(): void {
    const toAdd: string[] = [];
    for (const rootId of treeRootIds) {
      toAdd.push(rootId);
      toAdd.push(...getLoadedDescendantIds(rootId, treeNodes));
    }
    if (toAdd.length > 0) {
      applySelectionChange({ add: toAdd, remove: [] });
    }
  }

  function toggleMultiSelectMode(): void {
    if (multiSelectMode) {
      multiSelectMode = false;
      treeSelection = new Set();
      return;
    }
    multiSelectMode = true;
  }

  function openRemoveMultipleDialog(): void {
    if (get(wsStatus) !== "connected") return;
    removeMultipleError = "";
    removeMultipleDialogOpen = true;
  }

  function closeRemoveMultipleDialog(force = false): void {
    if (removeMultipleSubmitting && !force) return;
    removeMultipleDialogOpen = false;
    removeMultipleError = "";
  }

  async function confirmRemoveMultiple(): Promise<void> {
    if (get(wsStatus) !== "connected") {
      removeMultipleError =
        "WebSocket is disconnected. Please reconnect and try again.";
      return;
    }
    const rootId = treeRootIds[0] ?? null;
    const minimalIds = getMinimalAncestorSet(treeSelection, treeNodes, rootId);
    if (minimalIds.length === 0) {
      closeRemoveMultipleDialog(true);
      treeSelection = new Set();
      multiSelectMode = false;
      return;
    }
    removeMultipleSubmitting = true;
    removeMultipleError = "";
    try {
      await realtimeProvider.removeItems(minimalIds);
      treeSelection = new Set();
      multiSelectMode = false;
      closeRemoveMultipleDialog(true);
    } catch (error) {
      removeMultipleError =
        error instanceof Error ? error.message : "Failed to remove selection";
    } finally {
      removeMultipleSubmitting = false;
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
    if (!canDropToCanvas) {
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
    if (!canDropToCanvas) {
      return;
    }

    event.preventDefault();
    updateDragPreview(event);
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = "copy";
    }
  }

  function handleRightPanelDrop(event: DragEvent): void {
    if (!canDropToCanvas) {
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

  function toBoundWidgetTag(node: TreeNode): BoundWidgetTag {
    return {
      id: node.id,
      name: node.name,
      path: node.path,
      kind: node.kind,
      dataType: node.dataType,
    };
  }

  function createUnboundSourceNode(symbolId: string): BoundWidgetTag {
    return {
      id: `unbound-${symbolId}`,
      name: "Unbound",
      path: "-",
      kind: "folder",
    };
  }

  function getNodeDataById(nodeId: string): PlantAssetNodeData | null {
    const match = graphNodes.find((node) => node.id === nodeId);
    if (!match) return null;
    return match.data as PlantAssetNodeData;
  }

  function updateNodeData(
    nodeId: string,
    updater: (current: PlantAssetNodeData) => PlantAssetNodeData,
  ): void {
    graphNodes = graphNodes.map((node) => {
      if (node.id !== nodeId) return node;
      const current = node.data as PlantAssetNodeData;
      return {
        ...node,
        data: updater(current),
      };
    });
  }

  function getBindingSchema(
    data: PlantAssetNodeData,
    bindingKey: string,
  ): WidgetBindingSchema | undefined {
    const definition = resolveAssetDefinition(data.assetKind);
    return definition.bindings.find((binding) => binding.key === bindingKey);
  }

  function writeWidgetBindingValue(
    nodeId: string,
    bindingKey: string,
    value: TagScalarValue,
    tagId?: string,
  ): void {
    const data = getNodeDataById(nodeId);
    if (!data) return;

    const tags = data.bindings?.[bindingKey] ?? [];
    const target = tagId
      ? tags.find((tag) => tag.id === tagId && tag.kind === "tag")
      : tags.find((tag) => tag.kind === "tag");
    if (!target) {
      snackbarStore.warning(`Binding "${bindingKey}" is not configured.`);
      return;
    }

    void realtimeProvider.sendWriteValue(target.id, value);
  }

  function assignTagToBinding(
    nodeId: string,
    bindingKey: string,
    tagNode: TreeNode,
  ): void {
    if (tagNode.kind !== "tag") {
      snackbarStore.warning(
        "Only variable tags can be bound to widget fields.",
      );
      return;
    }

    updateNodeData(nodeId, (current) => {
      const schema = getBindingSchema(current, bindingKey);
      if (!schema) return current;

      const currentBindings = { ...(current.bindings ?? {}) };
      const prev = [...(currentBindings[bindingKey] ?? [])];
      const alreadyAdded = prev.some((tag) => tag.id === tagNode.id);
      const nextTag = toBoundWidgetTag(tagNode);
      const next = schema.multiple
        ? alreadyAdded
          ? prev
          : [...prev, nextTag]
        : [nextTag];

      currentBindings[bindingKey] = next;
      const nextPrimary = current.primaryBindingKey ?? schema.key;
      const sourceNode =
        currentBindings[nextPrimary]?.[0] ??
        createUnboundSourceNode(current.symbolId ?? current.title);

      return {
        ...current,
        bindings: currentBindings,
        primaryBindingKey: nextPrimary,
        sourceNode,
      };
    });
  }

  function removeTagFromBinding(
    nodeId: string,
    bindingKey: string,
    tagId: string,
  ): void {
    updateNodeData(nodeId, (current) => {
      const currentBindings = { ...(current.bindings ?? {}) };
      const next = (currentBindings[bindingKey] ?? []).filter(
        (tag) => tag.id !== tagId,
      );
      currentBindings[bindingKey] = next;

      const primaryKey = current.primaryBindingKey ?? bindingKey;
      const primaryTag = currentBindings[primaryKey]?.[0];
      const sourceNode =
        primaryTag ??
        createUnboundSourceNode(current.symbolId ?? current.title);
      return {
        ...current,
        bindings: currentBindings,
        sourceNode,
      };
    });
  }

  function resolveDroppedTreeNode(event: DragEvent): TreeNode | null {
    const id =
      event.dataTransfer?.getData("text/plain") || draggingNode?.id || null;
    if (!id) return null;
    return (
      treeNodes[id] ??
      (draggingNode && draggingNode.id === id ? draggingNode : null)
    );
  }

  function handleBindingDrop(
    event: DragEvent,
    nodeId: string,
    bindingKey: string,
  ): void {
    if (!canDropToCanvas) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    const dropped = resolveDroppedTreeNode(event);
    if (!dropped) return;
    assignTagToBinding(nodeId, bindingKey, dropped);
  }

  function focusGraphNodeInInspector(nodeId: string): void {
    inspectorDockVisible = true;
    graphNodes = graphNodes.map((node) => ({
      ...node,
      selected: node.id === nodeId,
    }));
  }

  function createInitialBindings(
    assetKind: string,
    droppedNode: TreeNode,
  ): Record<string, BoundWidgetTag[]> {
    if (droppedNode.kind !== "tag") return {};
    const definition = resolveAssetDefinition(assetKind);
    const primaryKey =
      definition.primaryBindingKey ?? definition.bindings[0]?.key;
    if (!primaryKey) return {};
    return {
      [primaryKey]: [toBoundWidgetTag(droppedNode)],
    };
  }

  function onDropAction(
    event: MouseEvent,
    node: TreeNode,
    assetKind: string,
  ): void {
    const position = getGraphPositionFromEvent(event);
    graphNodeCounter += 1;
    const nodeId = `asset-${assetKind}-${graphNodeCounter}`;
    const definition = resolveAssetDefinition(assetKind);
    const primaryBindingKey = definition.primaryBindingKey;
    const bindings = createInitialBindings(assetKind, node);
    const initialPrimaryTag = bindings[primaryBindingKey]?.[0];
    const canWritePrimary =
      definition.bindings.find((binding) => binding.key === primaryBindingKey)
        ?.access !== "read";

    const newNode: Node = {
      id: nodeId,
      type: "plantAsset",
      position,
      data: {
        symbolId: nodeId,
        assetKind,
        title: `${assetKind.toUpperCase()} ${graphNodeCounter}`,
        primaryBindingKey,
        bindings,
        sourceNode:
          initialPrimaryTag ??
          (node.kind === "tag"
            ? toBoundWidgetTag(node)
            : createUnboundSourceNode(nodeId)),
        onWriteValue: canWritePrimary
          ? (value: TagScalarValue) =>
              writeWidgetBindingValue(nodeId, primaryBindingKey, value)
          : undefined,
        onWriteBindingValue: (
          bindingKey: string,
          value: TagScalarValue,
          tagId?: string,
        ) => writeWidgetBindingValue(nodeId, bindingKey, value, tagId),
        onOpenBindingConfig: () => focusGraphNodeInInspector(nodeId),
      },
    };

    graphNodes = [
      ...graphNodes.map((existingNode) => ({
        ...existingNode,
        selected: false,
      })),
      { ...newNode, selected: true },
    ];
    inspectorDockVisible = true;
  }

  function handleConnect(connection: Connection): void {
    if (!canDropToCanvas) {
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
    if (rightPaneMode !== "view-editor") {
      return;
    }
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

  /** Path for dialog title: node path (id) with leading slash. Multiple roots: path is id from root to node. */
  function formatNamespaceBuilderPath(path: string): string {
    if (!path.trim()) return "/";
    return path.startsWith("/") ? path : `/${path}`;
  }

  /** Opens the Namespace Template Builder at root (used by toolbar). Sends "" as parentId for root-level bulk add. */
  function openNamespaceBuilderFromToolbar(): void {
    openNamespaceBuilderDialog("", "/");
  }

  /** Opens the Namespace Template Builder dialog (bulk add from YAML). parentId: "" for root, or folder id. parentDisplay: "/" for root, or path like "/Area_/Sub" for dialog title. */
  function openNamespaceBuilderDialog(
    parentId: string,
    parentDisplay: string,
  ): void {
    namespaceBuilderParentId = parentId;
    namespaceBuilderParentName = parentDisplay;
    namespaceBuilderDialogOpen = true;
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
    namespaceBuilderDialogOpen = false;
  }

  async function onNamespaceBuilderCreate(): Promise<void> {
    if (!namespaceBuilderRef) {
      snackbarStore.error("Namespace builder is not ready yet.");
      return;
    }
    if (
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
      const parentForBulk =
        namespaceBuilderParentId && namespaceBuilderParentId.trim() !== ""
          ? namespaceBuilderParentId
          : "/";
      await tagStreamClient.addBulkNamespace(
        parentForBulk,
        json,
        WS_ENDPOINT,
      );
      if (browser) {
        (
          window as unknown as { __lastNamespaceJson?: unknown }
        ).__lastNamespaceJson = json;
      }
      closeNamespaceBuilderDialog();
    } catch (err) {
      const msg =
        err instanceof Error && err.message
          ? err.message
          : "Bulk creation failed. Check connection and YAML.";
      snackbarStore.error(msg);
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

  const selectedGraphNode = $derived(
    graphNodes.find((node) => Boolean(node.selected)) ?? null,
  );
  const selectedGraphNodeData = $derived<PlantAssetNodeData | null>(
    selectedGraphNode ? (selectedGraphNode.data as PlantAssetNodeData) : null,
  );
  const selectedGraphWidgetDefinition = $derived(
    selectedGraphNodeData
      ? resolveAssetDefinition(selectedGraphNodeData.assetKind)
      : null,
  );

  onMount(() => {
    realtimeProvider.start();

    const handleDeleteKey = (event: KeyboardEvent) => {
      if (event.key !== "Delete") {
        return;
      }
      if (
        rightPaneMode !== "view-editor" ||
        canvasMode !== "edit" ||
        isTypingTarget(event.target)
      ) {
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
    if (viewsSearchDebounceTimer) {
      clearTimeout(viewsSearchDebounceTimer);
    }
    realtimeProvider.stop();
  });

  $effect(() => {
    if (
      rightPaneMode === "views-list" &&
      previousRightPaneMode !== "views-list"
    ) {
      void loadViewsList(viewsTableState);
    }
    previousRightPaneMode = rightPaneMode;
  });

  $effect(() => {
    if (!routeViewId) {
      loadedViewId = null;
      viewEditorError = "";
      viewEditorLoading = false;
      canvasMode = "edit";
      return;
    }
    if (loadedViewId === routeViewId) {
      return;
    }
    void loadViewEditor(routeViewId);
  });

  $effect(() => {
    realtimeProvider.setActive(
      rightPaneMode === "view-editor" && canvasMode === "play",
    );
  });

  const subscribedTagIds = $derived(getTrackedTagIds(graphNodes));

  $effect(() => {
    realtimeProvider.setDesiredIds(subscribedTagIds);
  });

  $effect(() => {
    if (rightPaneMode !== "view-editor" || canvasMode !== "play") return;
    const result = applyLiveValuesToGraphNodes(graphNodes, $tagValues);
    if (result.changed) {
      graphNodes = result.nodes;
    }
  });

  $effect(() => {
    const result = normalizePipeEdges(
      graphEdges,
      PIPE_EDGE_TYPE,
      PIPE_EDGE_STYLE,
    );
    if (result.changed) {
      graphEdges = result.edges;
    }
  });

</script>

<main class="flex h-dvh w-full flex-col gap-3 overflow-hidden bg-background p-4">
  <PageToolbar
    theme={$theme ?? "light"}
    {workspaceMode}
    onSelectWorkspaceMode={(mode) => (workspaceMode = mode)}
    onToggleTheme={toggleTheme}
    {username}
  />

  <div class="flex min-h-0 flex-1 gap-4">
    <section class="flex h-full w-[30%] min-w-[360px] flex-col gap-2">
      <div class="flex items-center justify-between rounded-md border border-border bg-card px-3 py-2">
        <h2 class="text-[15px] font-semibold text-foreground">Namespace Browser</h2>
        <div class="flex items-center gap-1">
          {#if multiSelectMode}
            <Button
              variant="icon"
              icon={Trash2}
              title="Remove selected"
              ariaLabel="Remove selected"
              disabled={selectionCount === 0}
              class="border-destructive/45 text-destructive hover:border-destructive/70 hover:bg-destructive/12 hover:text-destructive"
              onclick={openRemoveMultipleDialog}
            />
            <Button
              variant="icon"
              icon={CheckSquare}
              title="Select all"
              ariaLabel="Select all"
              onclick={selectAllSelection}
            />
            <Button
              variant="icon"
              icon={ListChecks}
              title="Multi-selection mode (click to exit)"
              ariaLabel="Multi-selection mode"
              selected={true}
              onclick={toggleMultiSelectMode}
            />
          {:else}
            <Button
              variant="icon"
              icon={Plus}
              title="Add variable or folder"
              ariaLabel="Add variable or folder"
              disabled={treeActionsDisabled}
              onclick={openTreeAddDialog}
            />
            <Button
              variant="icon"
              icon={Layers}
              title="Namespace Template Builder"
              ariaLabel="Namespace Template Builder"
              onclick={openNamespaceBuilderFromToolbar}
            />
            <Button
              variant="icon"
              icon={ListChecks}
              title="Multi-selection mode"
              ariaLabel="Multi-selection mode"
              onclick={toggleMultiSelectMode}
            />
          {/if}
        </div>
      </div>
      <VariableTree
        onNodeContextMenu={handleNodeContextMenu}
        onNodeDragStart={handleNodeDragStart}
        onNodeDragEnd={handleNodeDragEnd}
        onRootId={(id) => (treeRootId = id)}
        onCreateItem={createTreeItem}
        onEditMeta={editTreeMeta}
        websocketStatus={$wsStatus}
        realtimeEnabled={rightPaneMode === "view-editor" && canvasMode === "play"}
        liveTagValues={$tagValues}
        {multiSelectMode}
        selection={treeSelection}
        propagateDown={true}
        propagateUp={true}
        onSelectionChange={applySelectionChange}
        onTreeStateSnapshot={(nodes, rootIds) => {
          treeNodes = nodes;
          treeRootIds = rootIds;
        }}
      />
    </section>

    <section
      class="relative h-full flex-1 rounded-md border border-border bg-card p-0 text-sm text-muted-foreground"
      aria-label="Drop workspace"
      ondragover={handleRightPanelDragOver}
      ondrop={handleRightPanelDrop}
    >
      {#if rightPaneMode === "views-list"}
        <ViewsListPanel
          {views}
          total={viewsTotal}
          errorMessage={viewsLoadError}
          tableState={viewsTableState}
          searchValue={viewsSearchInput}
          loading={viewsLoading}
          createLoading={viewCreateLoading}
          busyId={viewActionBusyId}
          onCreate={() => void createViewInPlace()}
          onEdit={(view) => void goto(`/views/${view.id}`)}
          onRemove={(view) => openRemoveViewDialog(view)}
          onInlineUpdate={updateInlineViewFields}
          onSearchChange={onViewsSearchChange}
          onTableStateChange={onViewsTableStateChange}
        />
      {:else}
        <div class="flex h-full flex-col">
          <ViewEditorHeader
            view={activeView}
            {canvasMode}
            saving={viewSaveLoading}
            onBackToViewsList={() => goto("/views")}
            onToggleCanvasMode={toggleCanvasMode}
            onSave={() => void saveActiveViewCanvas()}
            onSetEntryPoint={() => void setActiveViewAsEntryPoint()}
          />
          {#if viewEditorError}
            <div class="flex min-h-0 flex-1 items-center justify-center px-6 py-10">
              <div class="max-w-xl px-6 py-5 text-center">
                <h2 class="text-base font-semibold text-foreground">Failed to load view</h2>
                <p class="mt-2 text-sm text-muted-foreground">{viewEditorError}</p>
                <div class="mt-4 flex justify-center">
                  <Button
                    variant="outline-accent"
                    label="Retry"
                    loading={viewEditorLoading}
                    loadingLabel="Retrying..."
                    disabled={!routeViewId || viewEditorLoading}
                    onclick={() => {
                      if (routeViewId) {
                        void loadViewEditor(routeViewId);
                      }
                    }}
                  />
                </div>
              </div>
            </div>
          {:else}
          <div class="relative flex min-h-0 flex-1">
            <div bind:this={graphHostRef} class="h-full flex-1">
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
                nodesDraggable={canDropToCanvas}
                elementsSelectable={canDropToCanvas}
                nodesConnectable={canDropToCanvas}
                selectionOnDrag={canDropToCanvas}
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

            {#if inspectorDockVisible}
              <aside
                class="h-full w-[340px] shrink-0 border-l border-border bg-card p-3"
                ondragover={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                  if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
                }}
                ondrop={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                }}
              >
                <div class="mb-3 flex items-center justify-between">
                  <h3 class="text-sm font-semibold text-foreground">
                    Graph Node Inspector
                  </h3>
                  <Button
                    variant="icon"
                    title="Hide inspector"
                    ariaLabel="Hide inspector"
                    label="×"
                    onclick={() => (inspectorDockVisible = false)}
                  />
                </div>

                {#if selectedGraphNodeData && selectedGraphWidgetDefinition}
                  {@const selectedGraphNodeId = selectedGraphNode?.id ?? ""}
                  <div class="space-y-3">
                    <div>
                      <label
                        for="node-title-input"
                        class="mb-1 block text-[10px] uppercase text-muted-foreground"
                        >Title</label
                      >
                      <Input
                        id="node-title-input"
                        class="w-full text-xs"
                        value={selectedGraphNodeData.title}
                        oninput={(event) => {
                          const target = event.currentTarget as HTMLInputElement;
                          updateNodeData(selectedGraphNodeId, (current) => ({
                            ...current,
                            title: sanitizeText(target.value, 80),
                          }));
                        }}
                      />
                    </div>

                    <div class="rounded border border-border p-2">
                      <p class="text-[10px] uppercase text-muted-foreground">
                        Widget
                      </p>
                      <p class="text-xs font-medium text-foreground">
                        {selectedGraphWidgetDefinition.label}
                      </p>
                    </div>

                    <div class="space-y-2">
                      {#each selectedGraphWidgetDefinition.bindings as binding}
                        {@const bindingTags =
                          selectedGraphNodeData.bindings?.[binding.key] ?? []}
                        <div class="rounded border border-border p-2">
                          <div class="mb-1 flex items-center justify-between">
                            <span class="font-medium text-foreground"
                              >{binding.label}</span
                            >
                            <span class="text-[10px] uppercase text-muted-foreground"
                              >{binding.access}</span
                            >
                          </div>
                          {#if binding.multiple}
                            <div
                              class="min-h-[38px] rounded border border-dashed border-border/60 bg-muted/50 px-2 py-1"
                              role="group"
                              ondragover={(event) => {
                                event.preventDefault();
                                event.stopPropagation();
                                if (event.dataTransfer)
                                  event.dataTransfer.dropEffect = "copy";
                              }}
                              ondrop={(event) =>
                                handleBindingDrop(
                                  event,
                                  selectedGraphNodeId,
                                  binding.key,
                                )}
                            >
                              {#if bindingTags.length === 0}
                                <span class="text-[10px] text-muted-foreground"
                                  >Drop tag(s) here</span
                                >
                              {:else}
                                <div class="flex flex-wrap gap-1">
                                  {#each bindingTags as tag (tag.id)}
                                    <span
                                      class="inline-flex items-center gap-1 rounded bg-primary/15 px-1.5 py-0.5 text-[10px] text-foreground"
                                    >
                                      <span class="max-w-[160px] truncate"
                                        >{tag.name}</span
                                      >
                                      <button
                                        type="button"
                                        class="text-[10px] opacity-70 hover:opacity-100"
                                        title="Remove binding"
                                        onclick={() =>
                                          removeTagFromBinding(
                                            selectedGraphNodeId,
                                            binding.key,
                                            tag.id,
                                          )}
                                      >
                                        ×
                                      </button>
                                    </span>
                                  {/each}
                                </div>
                              {/if}
                            </div>
                          {:else}
                            <Input
                              class="w-full text-[10px]"
                              readonly
                              value={bindingTags[0]?.path ?? ""}
                              placeholder="Drop tag here"
                              ondragover={(event) => {
                                event.preventDefault();
                                event.stopPropagation();
                                if (event.dataTransfer)
                                  event.dataTransfer.dropEffect = "copy";
                              }}
                              ondrop={(event) =>
                                handleBindingDrop(
                                  event,
                                  selectedGraphNodeId,
                                  binding.key,
                                )}
                            />
                          {/if}
                          {#if binding.required}
                            <p class="mt-1 text-[10px] text-muted-foreground">
                              Required
                            </p>
                          {/if}
                        </div>
                      {/each}
                    </div>
                  </div>
                {:else}
                  <div
                    class="flex h-[calc(100%-2rem)] items-center justify-center text-center text-sm text-muted-foreground"
                  >
                    Select a node in the graph to configure it.
                  </div>
                {/if}
              </aside>
            {:else}
              <div class="pointer-events-none absolute right-3 top-3 z-30">
                <div class="pointer-events-auto">
                  <Button
                    variant="outline-muted"
                    label="Show inspector"
                    title="Show inspector"
                    onclick={() => (inspectorDockVisible = true)}
                  />
                </div>
              </div>
            {/if}
          </div>
          {/if}
        </div>
      {/if}
    </section>
  </div>

  {#if dragPreview && draggingNode}
    <div
      class="pointer-events-none fixed z-40 rounded border border-border bg-card px-2 py-1 text-xs shadow-md"
      style={`left:${dragPreview.x}px;top:${dragPreview.y}px;`}
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

  <Dialog.Root bind:open={removeViewDialogOpen}>
    <Dialog.Content
      class="max-w-[420px]"
      showCloseButton={false}
      onInteractOutside={(event) => {
        event.preventDefault();
      }}
      onEscapeKeydown={(event) => {
        event.preventDefault();
      }}
    >
      <form
        class="flex flex-col gap-4"
        onsubmit={(event) => {
          event.preventDefault();
          void confirmRemoveView();
        }}
      >
        <Dialog.Header>
          <Dialog.Title>Confirm removal</Dialog.Title>
          <Dialog.Description>
            Remove view "{removeViewTarget?.name}"? This action cannot be
            undone.
          </Dialog.Description>
          {#if removeViewError}
            <p class="text-destructive text-xs/relaxed">{removeViewError}</p>
          {/if}
        </Dialog.Header>
        <Dialog.Footer class="border-border border-t pt-4">
          <Button
            variant="outline-muted"
            label="Cancel"
            title="Cancel"
            disabled={removeViewSubmitting}
            onclick={() => closeRemoveViewDialog()}
          />
          <Button
            type="submit"
            variant="filled-warn"
            label="Remove"
            loadingLabel="Removing..."
            loading={removeViewSubmitting}
            disabled={removeViewSubmitting || !removeViewTarget}
          />
        </Dialog.Footer>
      </form>
    </Dialog.Content>
  </Dialog.Root>

  <Dialog.Root bind:open={removeDialogOpen}>
    <Dialog.Content
      class="max-w-[420px]"
      showCloseButton={false}
      onInteractOutside={(event) => {
        event.preventDefault();
      }}
      onEscapeKeydown={(event) => {
        event.preventDefault();
      }}
    >
      <form
        class="flex flex-col gap-4"
        onsubmit={(event) => {
          event.preventDefault();
          void confirmRemoveTargetNode();
        }}
      >
        <Dialog.Header>
          <Dialog.Title>Confirm removal</Dialog.Title>
          <Dialog.Description>
            Remove "{removeTargetNode?.name}" ({removeTargetNode?.kind ===
            "folder"
              ? "folder"
              : "variable"})? This action cannot be undone.
          </Dialog.Description>
          {#if removeError}
            <p class="text-destructive text-xs/relaxed">{removeError}</p>
          {/if}
        </Dialog.Header>
        <Dialog.Footer class="border-border border-t pt-4">
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
        </Dialog.Footer>
      </form>
    </Dialog.Content>
  </Dialog.Root>

  <Dialog.Root bind:open={removeMultipleDialogOpen}>
    <Dialog.Content
      class="max-w-[420px]"
      showCloseButton={false}
      onInteractOutside={(event) => {
        event.preventDefault();
      }}
      onEscapeKeydown={(event) => {
        event.preventDefault();
      }}
    >
      <form
        class="flex flex-col gap-4"
        onsubmit={(event) => {
          event.preventDefault();
          void confirmRemoveMultiple();
        }}
      >
        <Dialog.Header>
          <Dialog.Title>Remove selection</Dialog.Title>
          <Dialog.Description>
            Remove selected item(s)? All descendants will also be removed. This
            action cannot be undone.
          </Dialog.Description>
          {#if removeMultipleError}
            <p class="text-destructive text-xs/relaxed">{removeMultipleError}</p>
          {/if}
        </Dialog.Header>
        <Dialog.Footer class="border-border border-t pt-4">
          <Button
            variant="outline-muted"
            label="Cancel"
            title="Cancel"
            disabled={removeMultipleSubmitting}
            onclick={() => closeRemoveMultipleDialog()}
          />
          <Button
            type="submit"
            variant="filled-warn"
            label="Remove"
            loadingLabel="Removing..."
            loading={removeMultipleSubmitting}
            disabled={removeMultipleSubmitting || $wsStatus !== "connected"}
          />
        </Dialog.Footer>
      </form>
    </Dialog.Content>
  </Dialog.Root>

  <Dialog.Root bind:open={namespaceBuilderDialogOpen}>
    <Dialog.Content
      class="flex h-[82vh] w-[92vw] max-w-[92vw] flex-col overflow-hidden sm:max-w-[1400px]"
      showCloseButton={false}
      onInteractOutside={(event) => {
        event.preventDefault();
      }}
      onEscapeKeydown={(event) => {
        event.preventDefault();
      }}
    >
      <div class="flex min-h-0 min-w-0 flex-1 flex-col">
        <Dialog.Header class="mb-2">
          <Dialog.Title>
            Namespace Template Builder — {namespaceBuilderParentName}
          </Dialog.Title>
        </Dialog.Header>
        <div class="min-h-0 min-w-0 flex-1 overflow-hidden">
          <NamespaceBuilder
            bind:this={namespaceBuilderRef}
            colorMode={$theme ?? "light"}
            createLoading={namespaceBuilderCreateLoading}
            onValidityChange={(v) => (namespaceBuilderValid = v)}
          />
        </div>
        <Dialog.Footer class="border-border mt-2 border-t pt-3">
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
        </Dialog.Footer>
      </div>
    </Dialog.Content>
  </Dialog.Root>
</main>
<div class="hidden">{@render children()}</div>
