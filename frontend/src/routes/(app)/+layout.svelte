<script lang="ts">
  import { afterNavigate, beforeNavigate, goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { browser } from "$app/environment";
  import { resolveTagStreamWsEndpoint } from "$lib/core/ws/resolve-ws-endpoint";
  import { onDestroy, onMount } from "svelte";
  import { SvelteMap, SvelteSet } from "svelte/reactivity";
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
  import VariableTree from "$lib/features/tree/components/VariableTree.svelte";
  import ContextMenu from "$lib/features/tree/components/ContextMenu.svelte";
  import GraphInspector from "$lib/features/workspace/components/GraphInspector.svelte";
  import PageToolbar from "$lib/features/workspace/components/PageToolbar.svelte";
  import WorkspaceConfirmDialogs from "$lib/features/workspace/components/WorkspaceConfirmDialogs.svelte";
  import {
    cloneForHistory,
    createGraphHistorySnapshot,
    snapshotSignature,
    buildNextEntityId,
    resolveInitialEditorViewport,
    type GraphHistorySnapshot,
  } from "$lib/features/workspace/layout-graph-history";
  import {
    applyNodeOrderByIds as applyNodeOrderByIdsForGraph,
    getOrderedNodeIdsByZ,
    getNextNodeZIndex,
    normalizeNodeZIndexes,
  } from "$lib/features/workspace/layout-graph-order";
  import {
    applyBoundMetadata,
    applyTreeSnapshotMetadata,
    applyVarMetaChanged,
    getParentIdFromNodePath,
    type BoundTagMetadata,
  } from "$lib/features/workspace/layout-graph-metadata";
  import NamespaceBuilder from "$lib/features/namespace-builder/components/NamespaceBuilder.svelte";
  import ViewsListPanel from "$lib/features/views/components/ViewsListPanel.svelte";
  import ViewEditorHeader from "$lib/features/views/components/ViewEditorHeader.svelte";
  import PlantAssetNode from "$lib/features/graph/components/PlantAssetNode.svelte";
  import ArrowConnectorEdge from "$lib/features/graph/components/ArrowConnectorEdge.svelte";
  import PipeConnectorEdge from "$lib/features/graph/components/PipeConnectorEdge.svelte";
  import {
    getRegisteredAssetDefinitions,
    resolveAssetDefinition,
  } from "$lib/features/graph/assets/registry";
  import {
    DEFAULT_NODE_PORT_OFFSETS,
    normalizeNodePortOffsets,
    type NodePortOffsets,
    type BoundWidgetTag,
    type PlantAssetNodeData,
    type WidgetEventBinding,
    type WidgetBindingSchema,
  } from "$lib/features/graph/assets/types";
  import {
    applyLiveValuesToGraphNodesAtIndexes,
    applyLiveValuesToGraphNodes,
    buildGraphLiveDependencyIndex,
    getTrackedTagIds,
  } from "$lib/features/graph/live-utils";
  import {
    DEFAULT_PIPE_CONNECTION_LINE_STYLE,
    applyArrowConnectorPatchToEdge,
    applyPipeConnectorPatchToEdge,
    applyConnectorStyleToEdge,
    createDefaultConnectorEdge,
    normalizeConnectorEdges,
    resolveArrowConnectorFromEdge,
    resolvePipeConnectorFromEdge,
    resolveConnectorStyle,
    type ArrowConnectorConfig,
    type ConnectorStyle,
    type PipeConnectorConfig,
  } from "$lib/features/graph/connectors";
  import {
    acceptsPrimaryBindingType,
    filterAssetDefinitionsByPrimaryType,
    isNodeCompatibleWithBinding,
  } from "$lib/features/graph/widget-type-compat";
  import {
    clearWidgetHandlers,
    registerWidgetHandlers,
    unregisterWidgetHandlers,
  } from "$lib/features/graph/widget-handlers";
  import { createPageTagRealtimeProvider } from "$lib/features/realtime/page-tag-realtime-provider";
  import { tagStreamClient } from "$lib/core/ws/tag-stream-client";
  import { snackbarStore } from "$lib/stores/snackbar";
  import { themeStore } from "$lib/stores/theme";
  import type { TagScalarValue } from "$lib/core/ws/types";
  import { ItemType, type VarDataType } from "@lirays/scada-proto";
  import type { VarMetaChanged } from "@lirays/scada-proto";
  import type { OnConnectStartParams } from "@xyflow/system";
  import type { LayoutData } from "./$types";
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
  import { sanitizeIdentifierLike } from "$lib/forms/sanitize";
  import {
    createView,
    deleteView,
    getEntryPointView,
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

  interface PendingNavigationIntent {
    href: string;
    replaceState: boolean;
  }
  interface GraphClipboardPayload {
    nodes: Node[];
    edges: Edge[];
  }

  type WorkspaceMode = "designer" | "runtime";
  let { children, data }: { children: import("svelte").Snippet; data: LayoutData } =
    $props();

  const WS_ENDPOINT = resolveTagStreamWsEndpoint();
  const PIPE_EDGE_STYLE = DEFAULT_PIPE_CONNECTION_LINE_STYLE;
  const GRAPH_HISTORY_LIMIT = 80;
  const theme = themeStore;
  const username = $derived(data.username ?? "admin");

  const realtimeProvider = createPageTagRealtimeProvider(WS_ENDPOINT);
  const nodeTypes = {
    plantAsset: PlantAssetNode,
  };
  const edgeTypes = {
    "connector-arrow": ArrowConnectorEdge,
    "connector-pipe": PipeConnectorEdge,
  };
  const wsStatus = realtimeProvider.status;
  const tagValues = realtimeProvider.values;

  let activeMenu = $state<ActiveMenuState | null>(null);
  let draggingNode = $state<TreeNode | null>(null);
  let dragPreview = $state<DragPreviewState | null>(null);
  let variableTreeRef: {
    openCreateDialog: (parentId?: string | null) => void;
    openMetadataDialog: (node: TreeNode) => void;
  } | null = null;
  const workspaceMode: WorkspaceMode = "designer";
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
  let viewsListRequestSerial = 0;
  let viewCreateLoading = $state(false);
  let viewActionBusyId = $state<string | null>(null);
  let loadedViewId = $state<string | null>(null);
  let viewEditorLoading = $state(false);
  let viewEditorRequestSerial = 0;
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
  let graphClipboard = $state<GraphClipboardPayload | null>(null);
  let graphClipboardPasteCount = $state(0);
  let graphHistory = $state<GraphHistorySnapshot[]>([]);
  let graphRedoHistory = $state<GraphHistorySnapshot[]>([]);
  let graphHistorySignature = $state("");
  let applyingGraphHistory = false;
  let graphHistoryViewId = $state<string | null>(null);
  let graphHistorySnapshotTimer: ReturnType<typeof setTimeout> | null = null;
  let knownWidgetHandlerSymbolIds = new SvelteSet<string>();
  let nodeDragInProgress = false;
  let graphViewport = $state<Viewport>({ x: 0, y: 0, zoom: 1 });
  let graphViewportApplyNonce = $state(0);
  let connectDragActive = $state(false);
  let connectStartNodeId = $state<string | null>(null);
  let removeDialogOpen = $state(false);
  let namespaceBuilderDialogOpen = $state(false);
  let namespaceBuilderRef: NamespaceBuilder | null = null;
  let namespaceBuilderValid = $state(true);
  let namespaceBuilderCreateLoading = $state(false);
  /** When opening from toolbar: root node id (or "" if tree not loaded); from folder context: folder id. */
  let namespaceBuilderParentId = "";
  let namespaceBuilderParentName = $state("Root");
  let removeTargetNode = $state<TreeNode | null>(null);
  let removeSubmitting = $state(false);
  let removeError = $state("");

  /** Multi-selection mode: show checkboxes and use global selection set instead of single select. */
  let multiSelectMode = $state(false);
  /** Set of node ids selected in multi-selection mode. */
  let treeSelection = new SvelteSet<string>();
  /** Snapshot of tree nodes from VariableTree for computing minimal delete set. */
  let treeNodes: Record<string, TreeNode> = {};
  let treeRootIds: string[] = [];
  let removeMultipleDialogOpen = $state(false);
  let removeMultipleSubmitting = $state(false);
  let removeMultipleError = $state("");
  let leaveViewDialogOpen = $state(false);
  let leaveViewSubmitting = $state(false);
  let pendingNavigationIntent = $state<PendingNavigationIntent | null>(null);
  let inspectorDockVisible = $state(true);
  let allowConfirmedLeaveNavigation = false;
  let bypassLeaveViewGuard = false;
  let previousTagValues = $state.raw<Record<string, TagScalarValue>>({});
  let graphLiveValuesRequestSerial = 0;
  const transparentDragImage: HTMLImageElement | null = browser
    ? new Image()
    : null;
  const canDropToCanvas = $derived(
    rightPaneMode === "view-editor" && canvasMode === "edit",
  );
  const activeView = $derived(views.find((view) => view.id === routeViewId) ?? null);
  const treeActionsDisabled = $derived($wsStatus !== "connected");
  const selectionCount = $derived(treeSelection.size);
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
          variableTreeRef?.openCreateDialog(context.node.id);
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
          variableTreeRef?.openMetadataDialog(context.node);
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
    return filterAssetDefinitionsByPrimaryType(
      getRegisteredAssetDefinitions(),
      context.node,
    )
      .map((definition) => ({
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
  const canvasMenuResolvers: MenuResolverByKind = {
    folder: (context) => buildDropAssetMenuOptions(context),
    tag: (context) => buildDropAssetMenuOptions(context),
  };
  const canvasMenuNode: TreeNode = {
    id: "canvas-root",
    parentId: null,
    name: "Canvas",
    path: "/",
    kind: "folder",
    hasChildren: false,
    childIds: null,
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

  function canEditGraphFromKeyboard(): boolean {
    return Boolean(routeViewId);
  }

  function resetGraphHistory(): void {
    if (graphHistorySnapshotTimer) {
      clearTimeout(graphHistorySnapshotTimer);
      graphHistorySnapshotTimer = null;
    }
    graphHistory = [];
    graphRedoHistory = [];
    graphHistorySignature = "";
  }

  function commitGraphHistorySnapshot(clearRedo = true): void {
    if (!canEditGraphFromKeyboard() || !loadedViewId || applyingGraphHistory) {
      return;
    }
    const snapshot = createGraphHistorySnapshot(graphNodes, graphEdges);
    const signature = snapshotSignature(snapshot);
    if (signature === graphHistorySignature) {
      return;
    }
    graphHistory = [...graphHistory, snapshot].slice(-GRAPH_HISTORY_LIMIT);
    graphHistorySignature = signature;
    if (clearRedo) {
      graphRedoHistory = [];
    }
  }

  function queueGraphHistorySnapshot(delayMs = 160): void {
    if (graphHistorySnapshotTimer) {
      clearTimeout(graphHistorySnapshotTimer);
      graphHistorySnapshotTimer = null;
    }
    graphHistorySnapshotTimer = setTimeout(() => {
      graphHistorySnapshotTimer = null;
      commitGraphHistorySnapshot(true);
    }, delayMs);
  }

  function flushPendingGraphHistorySnapshot(): void {
    if (!graphHistorySnapshotTimer) {
      return;
    }
    clearTimeout(graphHistorySnapshotTimer);
    graphHistorySnapshotTimer = null;
    commitGraphHistorySnapshot(true);
  }

  function restoreGraphFromSnapshot(snapshot: GraphHistorySnapshot): void {
    applyingGraphHistory = true;
    const cloned = cloneForHistory(snapshot);
    graphHistorySignature = snapshotSignature(cloned);
    clearWidgetHandlers();
    graphNodes = hydrateCanvasNodes(cloned.nodes);
    graphEdges = normalizeConnectorEdges(cloned.edges).edges;
    syncGraphCounters(graphNodes, graphEdges);
    previousTagValues = {};
    applyingGraphHistory = false;
  }

  function undoGraphChange(): void {
    flushPendingGraphHistorySnapshot();
    if (!canEditGraphFromKeyboard() || graphHistory.length <= 1) {
      return;
    }
    const current = graphHistory[graphHistory.length - 1];
    const previous = graphHistory[graphHistory.length - 2];
    graphRedoHistory = [...graphRedoHistory, cloneForHistory(current)];
    graphHistory = graphHistory.slice(0, -1);
    restoreGraphFromSnapshot(previous);
  }

  function redoGraphChange(): void {
    flushPendingGraphHistorySnapshot();
    if (!canEditGraphFromKeyboard() || graphRedoHistory.length === 0) {
      return;
    }
    const next = graphRedoHistory[graphRedoHistory.length - 1];
    graphRedoHistory = graphRedoHistory.slice(0, -1);
    graphHistory = [...graphHistory, cloneForHistory(next)].slice(
      -GRAPH_HISTORY_LIMIT,
    );
    restoreGraphFromSnapshot(next);
  }

  function cloneSelectedGraphNodesToClipboard(): void {
    if (!canEditGraphFromKeyboard()) {
      return;
    }
    const selectedNodes = graphNodes.filter((node) => Boolean(node.selected));
    if (selectedNodes.length === 0) {
      return;
    }
    const selectedIds = new SvelteSet(selectedNodes.map((node) => node.id));
    const selectedEdges = graphEdges.filter(
      (edge) => selectedIds.has(edge.source) && selectedIds.has(edge.target),
    );
    graphClipboard = {
      nodes: cloneForHistory(selectedNodes),
      edges: cloneForHistory(selectedEdges),
    };
    graphClipboardPasteCount = 0;
  }

  function pasteGraphNodesFromClipboard(): void {
    if (!canEditGraphFromKeyboard() || !graphClipboard) {
      return;
    }
    if (graphClipboard.nodes.length === 0) {
      return;
    }

    graphClipboardPasteCount += 1;
    const positionOffset = 24 * graphClipboardPasteCount;
    const sourceNodes = cloneForHistory(graphClipboard.nodes);
    const sourceEdges = cloneForHistory(graphClipboard.edges);
    const idMap = new SvelteMap<string, string>();
    const pastedNodes: Node[] = [];
    let nextZIndex = getNextNodeZIndex(graphNodes);

    for (const sourceNode of sourceNodes) {
      graphNodeCounter += 1;
      const nextId = buildNextEntityId(sourceNode.id, graphNodeCounter);
      idMap.set(sourceNode.id, nextId);

      const nodeData = sourceNode.data as PlantAssetNodeData;
      const nextData: PlantAssetNodeData = {
        ...nodeData,
        symbolId: nextId,
        sourceNode:
          nodeData.sourceNode?.kind === "tag"
            ? nodeData.sourceNode
            : createUnboundSourceNode(nextId),
      };

      const pastedNode = enforceNodeMinSize({
        ...sourceNode,
        id: nextId,
        zIndex: nextZIndex,
        selected: true,
        position: {
          x: sourceNode.position.x + positionOffset,
          y: sourceNode.position.y + positionOffset,
        },
        data: hydratePlantAssetNodeData(nextId, nextData),
      });
      pastedNodes.push(pastedNode);
      nextZIndex += 1;
    }

    const pastedEdges: Edge[] = [];
    for (const sourceEdge of sourceEdges) {
      const nextSource = idMap.get(sourceEdge.source);
      const nextTarget = idMap.get(sourceEdge.target);
      if (!nextSource || !nextTarget) {
        continue;
      }
      graphEdgeCounter += 1;
      pastedEdges.push({
        ...sourceEdge,
        id: `pipe-${graphEdgeCounter}`,
        source: nextSource,
        target: nextTarget,
        selected: false,
      });
    }
    const normalizedPastedEdges = normalizeConnectorEdges(pastedEdges).edges;

    graphNodes = [
      ...graphNodes.map((node) => ({
        ...node,
        selected: false,
      })),
      ...pastedNodes,
    ];
    graphEdges = [
      ...graphEdges.map((edge) => ({
        ...edge,
        selected: false,
      })),
      ...normalizedPastedEdges,
    ];
    inspectorDockVisible = true;
    queueGraphHistorySnapshot(0);
  }

  function getAssetMinDimensions(assetKind: string): {
    width: number;
    height: number;
  } {
    const definition = resolveAssetDefinition(assetKind);
    return {
      width: definition.minWidth ?? 240,
      height: definition.minHeight ?? 160,
    };
  }

  function enforceNodeMinSize(node: Node): Node {
    if (node.type !== "plantAsset") {
      return node;
    }

    const data = node.data as PlantAssetNodeData | undefined;
    const assetKind = data?.assetKind;
    if (!assetKind) {
      return node;
    }

    const min = getAssetMinDimensions(assetKind);
    const nextWidth =
      typeof node.width === "number" ? Math.max(node.width, min.width) : min.width;
    const nextHeight =
      typeof node.height === "number" ? Math.max(node.height, min.height) : min.height;

    if (node.width === nextWidth && node.height === nextHeight) {
      return node;
    }

    return {
      ...node,
      width: nextWidth,
      height: nextHeight,
    };
  }

  function hydratePlantAssetNodeData(
    nodeId: string,
    raw: PlantAssetNodeData,
  ): PlantAssetNodeData {
    const persistedRaw = { ...raw };
    delete persistedRaw.liveValue;
    delete persistedRaw.liveValues;
    delete persistedRaw.onWriteValue;
    delete persistedRaw.onWriteBindingValue;
    delete persistedRaw.onWidgetEvent;
    delete persistedRaw.onOpenBindingConfig;
    const symbolId = raw.symbolId ?? nodeId;
    const definition = resolveAssetDefinition(raw.assetKind);
    const primaryBindingKey = raw.primaryBindingKey ?? definition.primaryBindingKey ?? "";
    const primaryBinding = definition.bindings.find(
      (binding) => binding.key === primaryBindingKey,
    );
    const canWritePrimary =
      Boolean(primaryBinding?.key) && primaryBinding?.access !== "read";

    registerWidgetHandlers(symbolId, {
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
    });

    const widgetConfig = {
      ...(definition.defaultConfig ?? {}),
      ...(raw.widgetConfig ?? {}),
    };

    return {
      ...persistedRaw,
      symbolId,
      graphReadOnly: !canDropToCanvas,
      primaryBindingKey,
      interactionMode: "editor",
      eventBindings: sanitizeEventBindings(raw.assetKind, raw.eventBindings ?? []),
      widgetConfig,
      portOffsets: normalizeNodePortOffsets(raw.portOffsets),
    };
  }

  function hydrateCanvasNodes(nodes: Node[]): Node[] {
    const result = normalizeNodeZIndexes(nodes.map((node) => {
      if (node.type !== "plantAsset") {
        return node;
      }

      const data = node.data as PlantAssetNodeData;
      return {
        ...enforceNodeMinSize(node),
        data: hydratePlantAssetNodeData(node.id, data),
      };
    }));
    return result.nodes;
  }

  function replaceViewInState(nextView: ScadaView): void {
    if (views.some((view) => view.id === nextView.id)) {
      views = views.map((view) => (view.id === nextView.id ? nextView : view));
      return;
    }
    views = [...views, nextView];
  }

  async function loadViewsList(tableState = viewsTableState): Promise<void> {
    const requestId = ++viewsListRequestSerial;
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
      if (requestId !== viewsListRequestSerial) {
        return;
      }
      views = result.items;
      viewsTotal = result.total;
    } catch (error) {
      if (requestId !== viewsListRequestSerial) {
        return;
      }
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
      if (requestId === viewsListRequestSerial) {
        viewsLoading = false;
      }
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
    const requestId = ++viewEditorRequestSerial;
    viewEditorLoading = true;
    viewEditorError = "";
    viewActionBusyId = viewId;
    try {
      const view = await getView(viewId);
      if (requestId !== viewEditorRequestSerial) {
        return;
      }
      const canvas = deserializeCanvasState(view.canvas_json);
      clearWidgetHandlers();
      graphNodes = hydrateCanvasNodes(canvas.nodes);
      refreshGraphBindingMetadataFromTreeSnapshot();
      void refreshGraphLiveValuesFromBackend(requestId);
      void refreshGraphBindingMetadataFromBackend(requestId);
      graphEdges = normalizeConnectorEdges(canvas.edges).edges;
      graphViewport = resolveInitialEditorViewport(
        canvas.viewport,
        graphNodes,
        graphHostRef?.clientWidth ?? 1280,
        graphHostRef?.clientHeight ?? 720,
      );
      graphViewportApplyNonce += 1;
      previousTagValues = {};
      syncGraphCounters(graphNodes, graphEdges);
      replaceViewInState(view);
      canvasMode = "edit";
      loadedViewId = view.id;
    } catch (error) {
      if (requestId !== viewEditorRequestSerial) {
        return;
      }
      clearWidgetHandlers();
      graphNodes = [];
      graphEdges = [];
      graphViewport = { x: 0, y: 0, zoom: 1 };
      graphViewportApplyNonce += 1;
      previousTagValues = {};
      loadedViewId = null;
      viewEditorError =
        error instanceof Error && error.message
          ? error.message
          : "Unable to load this view.";
      snackbarStore.error(
        "Failed to open view",
      );
    } finally {
      if (requestId === viewEditorRequestSerial) {
        viewEditorLoading = false;
        viewActionBusyId = null;
      }
    }
  }

  function randomViewName(): string {
    const suffix = (browser && "randomUUID" in crypto
      ? crypto.randomUUID()
      : Math.random().toString(36).slice(2, 10)
    ).slice(0, 8);
    return `View ${suffix}`;
  }

  function extractDuplicateViewMessage(error: unknown): string | null {
    if (!(error instanceof Error)) {
      return null;
    }
    const message = error.message?.trim() ?? "";
    if (!message) {
      return null;
    }
    return /view name .*already exists/i.test(message) ? message : null;
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
      const duplicateMessage = extractDuplicateViewMessage(error);
      snackbarStore.error(duplicateMessage ?? "Failed to create view");
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
        clearWidgetHandlers();
        graphNodes = [];
        graphEdges = [];
        graphViewport = { x: 0, y: 0, zoom: 1 };
        loadedViewId = null;
        bypassLeaveViewGuard = true;
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
      bypassLeaveViewGuard = false;
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
      const duplicateMessage = extractDuplicateViewMessage(error);
      snackbarStore.error(duplicateMessage ?? "Failed to save view");
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
    } catch {
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
      const duplicateMessage = extractDuplicateViewMessage(error);
      snackbarStore.error(duplicateMessage ?? "Failed to update view");
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
    for (const id of payload.remove) treeSelection.delete(id);
    for (const id of payload.add) treeSelection.add(id);
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
      treeSelection.clear();
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
      treeSelection.clear();
      multiSelectMode = false;
      return;
    }
    removeMultipleSubmitting = true;
    removeMultipleError = "";
    try {
      await realtimeProvider.removeItems(minimalIds);
      treeSelection.clear();
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

  function handleCanvasContextMenu(event: MouseEvent): void {
    if (!canDropToCanvas) {
      return;
    }

    const target = event.target;
    if (target instanceof HTMLElement && target.closest(".svelte-flow__node")) {
      return;
    }

    event.preventDefault();
    openContextMenu(
      event,
      { node: canvasMenuNode, event, kind: "drop" },
      canvasMenuResolvers,
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

  function handleCanvasNodeDragStart(): void {
    nodeDragInProgress = true;
    if (graphHistorySnapshotTimer) {
      clearTimeout(graphHistorySnapshotTimer);
      graphHistorySnapshotTimer = null;
    }
  }

  function handleCanvasNodeDragStop(): void {
    nodeDragInProgress = false;
    queueGraphHistorySnapshot(0);
  }

  function toBoundWidgetTag(node: TreeNode): BoundWidgetTag {
    return {
      id: node.id,
      name: node.name,
      path: node.path,
      kind: node.kind,
      dataType: node.dataType,
      unit: node.unit,
      min: node.min,
      max: node.max,
      options: node.options,
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

  function applyNodeOrderByIds(orderedIds: string[]): void {
    const result = applyNodeOrderByIdsForGraph(graphNodes, orderedIds);
    if (result.changed) {
      graphNodes = result.nodes;
    }
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

  function normalizeNodeDimension(value: number, min: number): number {
    if (!Number.isFinite(value)) {
      return min;
    }
    return Math.max(min, Math.round(value));
  }

  function updateNodeDimensions(
    nodeId: string,
    dimensions: { width?: number; height?: number },
  ): void {
    if (!nodeId) {
      return;
    }

    graphNodes = graphNodes.map((node) => {
      if (node.id !== nodeId || node.type !== "plantAsset") {
        return node;
      }

      const data = node.data as PlantAssetNodeData;
      const min = getAssetMinDimensions(data.assetKind);
      const currentWidth =
        typeof node.width === "number"
          ? normalizeNodeDimension(node.width, min.width)
          : min.width;
      const currentHeight =
        typeof node.height === "number"
          ? normalizeNodeDimension(node.height, min.height)
          : min.height;
      const nextWidth =
        typeof dimensions.width === "number"
          ? normalizeNodeDimension(dimensions.width, min.width)
          : currentWidth;
      const nextHeight =
        typeof dimensions.height === "number"
          ? normalizeNodeDimension(dimensions.height, min.height)
          : currentHeight;

      if (nextWidth === currentWidth && nextHeight === currentHeight) {
        return node;
      }

      return {
        ...node,
        width: nextWidth,
        height: nextHeight,
      };
    });
  }

  function updateNodePortOffsets(
    nodeId: string,
    offsets: Partial<NodePortOffsets>,
  ): void {
    if (!nodeId) {
      return;
    }

    updateNodeData(nodeId, (current) => ({
      ...current,
      portOffsets: normalizeNodePortOffsets({
        ...(current.portOffsets ?? DEFAULT_NODE_PORT_OFFSETS),
        ...offsets,
      }),
    }));
  }

  function updateNodeEventBindings(
    nodeId: string,
    bindings: WidgetEventBinding[],
  ): void {
    updateNodeData(nodeId, (current) => ({
      ...current,
      eventBindings: sanitizeEventBindings(current.assetKind, bindings),
    }));
  }

  function updateNodeWidgetConfig(
    nodeId: string,
    widgetConfig: Record<string, unknown>,
  ): void {
    updateNodeData(nodeId, (current) => ({
      ...current,
      widgetConfig,
    }));
  }

  function updateEdgeConnectorStyle(edgeId: string, style: ConnectorStyle): void {
    if (!edgeId) {
      return;
    }
    graphEdges = graphEdges.map((edge) =>
      edge.id === edgeId ? applyConnectorStyleToEdge(edge, style) : edge,
    );
    queueGraphHistorySnapshot(0);
  }

  function updateEdgeArrowConnectorConfig(
    edgeId: string,
    patch: Partial<ArrowConnectorConfig>,
  ): void {
    if (!edgeId) {
      return;
    }
    graphEdges = graphEdges.map((edge) =>
      edge.id === edgeId ? applyArrowConnectorPatchToEdge(edge, patch) : edge,
    );
    queueGraphHistorySnapshot(0);
  }

  function updateEdgePipeConnectorConfig(
    edgeId: string,
    patch: Partial<PipeConnectorConfig>,
  ): void {
    if (!edgeId) {
      return;
    }
    graphEdges = graphEdges.map((edge) =>
      edge.id === edgeId ? applyPipeConnectorPatchToEdge(edge, patch) : edge,
    );
    queueGraphHistorySnapshot(0);
  }

  function sanitizeEventBindings(
    assetKind: string,
    bindings: WidgetEventBinding[],
  ): WidgetEventBinding[] {
    const supported = resolveAssetDefinition(assetKind).supportedEvents ?? [];
    if (supported.length === 0) {
      return [];
    }

    const seen = new SvelteSet<WidgetEventBinding["on"]>();
    const sanitized: WidgetEventBinding[] = [];
    for (const binding of bindings) {
      if (!supported.includes(binding.on) || seen.has(binding.on)) {
        continue;
      }
      seen.add(binding.on);
      sanitized.push(binding);
    }
    return sanitized;
  }

  function applyVarMetaChangedToGraphNodes(changes: VarMetaChanged[]): void {
    const result = applyVarMetaChanged(graphNodes, changes);
    if (result.changed) {
      graphNodes = result.nodes;
    }
  }

  function refreshGraphBindingMetadataFromTreeSnapshot(): void {
    const result = applyTreeSnapshotMetadata(graphNodes, treeNodes);
    if (result.changed) {
      graphNodes = result.nodes;
    }
  }

  async function refreshGraphLiveValuesFromBackend(
    requestId?: number,
  ): Promise<void> {
    if (canvasMode === "play" || graphNodes.length === 0) {
      return;
    }

    const trackedTagIds = getTrackedTagIds(graphNodes);
    if (trackedTagIds.length === 0) {
      return;
    }

    const requestSerial = ++graphLiveValuesRequestSerial;
    try {
      const values = await tagStreamClient.getValues(trackedTagIds, WS_ENDPOINT);
      if (requestSerial !== graphLiveValuesRequestSerial) {
        return;
      }
      if (
        requestId !== undefined &&
        requestId !== viewEditorRequestSerial
      ) {
        return;
      }
      const result = applyLiveValuesToGraphNodes(graphNodes, values);
      if (result.changed) {
        graphNodes = result.nodes;
      }
    } catch {
      // Keep editor usable even if live-value refresh fails transiently.
    }
  }

  async function fetchLatestMetadataForTagIds(
    tagIds: string[],
  ): Promise<Map<string, BoundTagMetadata>> {
    const result = new SvelteMap<string, BoundTagMetadata>();
    if (tagIds.length === 0) {
      return result;
    }

    const tagIdSet = new SvelteSet(tagIds);
    const parentIds = new SvelteSet<string | null>();
    for (const tagId of tagIds) {
      parentIds.add(getParentIdFromNodePath(tagId));
    }

    const requests = [...parentIds].map((parentId) =>
      tagStreamClient
        .listChildren(parentId ?? undefined, WS_ENDPOINT)
        .then((payload) => ({ parentId, payload })),
    );
    const responses = await Promise.allSettled(requests);

    for (const response of responses) {
      if (response.status !== "fulfilled") {
        continue;
      }
      for (const variable of response.value.payload.variables ?? []) {
        if (!tagIdSet.has(variable.id)) {
          continue;
        }
        result.set(variable.id, {
          min: variable.min ?? undefined,
          max: variable.max ?? undefined,
          unit: variable.unit ?? undefined,
          options: variable.options ?? [],
        });
      }
    }

    return result;
  }

  async function refreshGraphBindingMetadataFromBackend(
    requestId?: number,
  ): Promise<void> {
    const tagIds = getTrackedTagIds(graphNodes);
    if (tagIds.length === 0) {
      return;
    }
    try {
      const latest = await fetchLatestMetadataForTagIds(tagIds);
      if (
        requestId !== undefined &&
        requestId !== viewEditorRequestSerial
      ) {
        return;
      }
      applyBoundMetadataToGraphNodes(latest);
    } catch {
      // Keep editor usable even if metadata refresh fails transiently.
    }
  }

  function applyBoundMetadataToGraphNodes(
    byTagId: Map<string, BoundTagMetadata>,
  ): void {
    const result = applyBoundMetadata(graphNodes, byTagId);
    if (result.changed) {
      graphNodes = result.nodes;
    }
  }

  async function refreshGraphBindingMetadataBeforePlay(): Promise<void> {
    refreshGraphBindingMetadataFromTreeSnapshot();
    await refreshGraphBindingMetadataFromBackend();
  }

  function handleTreeStateSnapshot(
    nodes: Record<string, TreeNode>,
    rootIds: string[],
  ): void {
    treeNodes = nodes;
    treeRootIds = rootIds;
    refreshGraphBindingMetadataFromTreeSnapshot();
    void refreshGraphLiveValuesFromBackend();
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
      if (!isNodeCompatibleWithBinding(tagNode, schema)) {
        snackbarStore.warning(
          `"${schema.label}" only accepts: ${(schema.dataTypes ?? []).join(", ")}`,
        );
        return current;
      }

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
      const definition = resolveAssetDefinition(current.assetKind);
      const sourceNode =
        currentBindings[nextPrimary]?.[0] ??
        createUnboundSourceNode(current.symbolId ?? current.title);
      const isPrimaryBinding = schema.key === nextPrimary;
      const hasLabel = current.title.trim().length > 0;
      const isDefaultLabel =
        current.title === definition.label ||
        current.title === `${definition.label} ${extractTrailingCounter(nodeId)}` ||
        /^[A-Z0-9_]+ \d+$/.test(current.title);
      const shouldUseVariableName = isPrimaryBinding && (!hasLabel || isDefaultLabel);

      return {
        ...current,
        title: shouldUseVariableName ? tagNode.name : current.title,
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
    graphEdges = graphEdges.map((edge) => ({
      ...edge,
      selected: false,
    }));
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
    if (!acceptsPrimaryBindingType(resolveAssetDefinition(assetKind), node)) {
      const definition = resolveAssetDefinition(assetKind);
      const primaryBinding = definition.bindings.find(
        (binding) => binding.key === definition.primaryBindingKey,
      );
      if (primaryBinding?.dataTypes?.length) {
        snackbarStore.warning(
          `"${definition.label}" expects ${primaryBinding.dataTypes.join(", ")} on "${primaryBinding.label}".`,
        );
      }
      return;
    }

    const position = getGraphPositionFromEvent(event);
    graphNodeCounter += 1;
    const nodeId = `asset-${assetKind}-${graphNodeCounter}`;
    const definition = resolveAssetDefinition(assetKind);
    const primaryBindingKey =
      definition.primaryBindingKey ?? definition.bindings[0]?.key ?? "";
    const bindings = createInitialBindings(assetKind, node);
    const initialPrimaryTag = primaryBindingKey
      ? bindings[primaryBindingKey]?.[0]
      : undefined;
    const primaryBinding = definition.bindings.find(
      (binding) => binding.key === primaryBindingKey,
    );
    const canWritePrimary =
      Boolean(primaryBinding?.key) && primaryBinding?.access !== "read";
    const defaultLabel =
      node.kind === "tag"
        ? node.name
        : `${definition.label} ${graphNodeCounter}`;

    const newNode: Node = {
      id: nodeId,
      type: "plantAsset",
      position,
      zIndex: getNextNodeZIndex(graphNodes),
      ...getAssetMinDimensions(assetKind),
      data: {
        symbolId: nodeId,
        assetKind,
        title: defaultLabel.slice(0, 80),
        graphReadOnly: !canDropToCanvas,
        primaryBindingKey,
        interactionMode: "editor",
        eventBindings: [],
        widgetConfig: { ...(definition.defaultConfig ?? {}) },
        portOffsets: { ...DEFAULT_NODE_PORT_OFFSETS },
        bindings,
        sourceNode:
          initialPrimaryTag ??
          createUnboundSourceNode(nodeId),
      },
    };

    registerWidgetHandlers(nodeId, {
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
    });

    graphNodes = [
      ...graphNodes.map((existingNode) => ({
        ...existingNode,
        selected: false,
      })),
      { ...newNode, selected: true },
    ];
    inspectorDockVisible = true;
    queueGraphHistorySnapshot(0);
  }

  function handleConnect(connection: Connection): void {
    if (!canDropToCanvas) {
      return;
    }

    const orientedConnection = orientConnectionFromDragStart(connection);

    graphEdgeCounter += 1;
    graphEdges = addEdge(
      createDefaultConnectorEdge(orientedConnection, `pipe-${graphEdgeCounter}`),
      graphEdges,
    );
    queueGraphHistorySnapshot(0);
  }

  function orientConnectionFromDragStart(connection: Connection): Connection {
    const sourceHandleKind = handleKindFromId(connection.sourceHandle);
    const targetHandleKind = handleKindFromId(connection.targetHandle);
    const shouldSwapByHandleKinds =
      sourceHandleKind === "target" ||
      targetHandleKind === "source" ||
      (sourceHandleKind === "target" && targetHandleKind === "source");
    if (shouldSwapByHandleKinds) {
      return {
        source: connection.target,
        sourceHandle: connection.targetHandle ?? null,
        target: connection.source,
        targetHandle: connection.sourceHandle ?? null,
      };
    }

    const startNodeId = connectStartNodeId;
    if (!startNodeId) {
      return connection;
    }

    if (connection.source === startNodeId) {
      return connection;
    }

    if (connection.target === startNodeId) {
      return {
        source: connection.target,
        sourceHandle: connection.targetHandle ?? null,
        target: connection.source,
        targetHandle: connection.sourceHandle ?? null,
      };
    }

    return connection;
  }

  function handleKindFromId(handleId: string | null | undefined): string | null {
    if (typeof handleId !== "string") {
      return null;
    }
    if (handleId.startsWith("source-")) {
      return "source";
    }
    if (handleId.startsWith("target-")) {
      return "target";
    }
    return null;
  }

  async function toggleCanvasMode(): Promise<void> {
    if (rightPaneMode !== "view-editor") {
      return;
    }
    const nextMode: CanvasMode = canvasMode === "edit" ? "play" : "edit";
    canvasMode = nextMode;

    if (nextMode === "play") {
      // Pull latest metadata from both local tree cache and backend before play.
      await refreshGraphBindingMetadataBeforePlay();
      // Ensure the latest tracked ids are pushed right when entering play.
      realtimeProvider.setDesiredIds(getTrackedTagIds(graphNodes));
      realtimeProvider.setActive(true);
      return;
    }

    realtimeProvider.setActive(false);
    void refreshGraphLiveValuesFromBackend();
  }

  function toggleTheme(): void {
    themeStore.update((current) => (current === "dark" ? "light" : "dark"));
  }

  async function navigateWorkspaceMode(mode: WorkspaceMode): Promise<void> {
    if (mode === "designer") {
      await goto("/views");
      return;
    }

    try {
      const entryPoint = await getEntryPointView();
      await goto(`/runtime/${entryPoint.id}`);
    } catch {
      await goto("/runtime");
    }
  }

  /** Opens the Add Variable/Folder dialog (same as tree context menu) at root. */
  function openTreeAddDialog(): void {
    variableTreeRef?.openCreateDialog(null);
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

    const removedSymbolIds = graphNodes
      .filter((node) => selectedIds.includes(node.id) && node.type === "plantAsset")
      .map((node) => (node.data as PlantAssetNodeData).symbolId ?? node.id);
    for (const symbolId of removedSymbolIds) {
      unregisterWidgetHandlers(symbolId);
    }

    graphNodes = graphNodes.filter((node) => !selectedIds.includes(node.id));
    graphEdges = graphEdges.filter(
      (edge) =>
        !selectedIds.includes(edge.source) &&
        !selectedIds.includes(edge.target) &&
        !selectedEdgeIds.includes(edge.id),
    );
    queueGraphHistorySnapshot(0);
  }

  function handleGraphShortcuts(event: KeyboardEvent): void {
    if (!routeViewId) {
      return;
    }

    const isModPressed = event.ctrlKey || event.metaKey;
    const key = event.key.toLowerCase();
    const targetIsTyping = isTypingTarget(event.target);

    if (event.key === "Delete" || event.key === "Backspace") {
      if (targetIsTyping) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      deleteSelectedGraphNodes();
      return;
    }

    if (!isModPressed) {
      return;
    }

    if (key === "c") {
      if (targetIsTyping) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      cloneSelectedGraphNodesToClipboard();
      return;
    }

    if (key === "v") {
      if (targetIsTyping) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      pasteGraphNodesFromClipboard();
      return;
    }

    if (key === "z") {
      if (targetIsTyping) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      if (event.shiftKey) {
        redoGraphChange();
        return;
      }
      undoGraphChange();
      return;
    }

    if (key === "y") {
      if (targetIsTyping) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      redoGraphChange();
    }
  }

  function reorderSelectedGraphNodes(
    direction: "back" | "backward" | "forward" | "front",
  ): void {
    if (canvasMode !== "edit") {
      return;
    }

    const selectedIds = new SvelteSet(
      graphNodes
        .filter((node) => Boolean(node.selected))
        .map((node) => node.id),
    );
    if (selectedIds.size === 0) {
      return;
    }

    const orderedIds = getOrderedNodeIdsByZ(graphNodes);

    if (direction === "front") {
      const next = [
        ...orderedIds.filter((id) => !selectedIds.has(id)),
        ...orderedIds.filter((id) => selectedIds.has(id)),
      ];
      applyNodeOrderByIds(next);
      return;
    }

    if (direction === "back") {
      const next = [
        ...orderedIds.filter((id) => selectedIds.has(id)),
        ...orderedIds.filter((id) => !selectedIds.has(id)),
      ];
      applyNodeOrderByIds(next);
      return;
    }

    const shifted = [...orderedIds];
    if (direction === "forward") {
      for (let index = shifted.length - 2; index >= 0; index -= 1) {
        const current = shifted[index];
        const next = shifted[index + 1];
        if (selectedIds.has(current) && !selectedIds.has(next)) {
          shifted[index] = next;
          shifted[index + 1] = current;
        }
      }
      applyNodeOrderByIds(shifted);
      return;
    }

    for (let index = 1; index < shifted.length; index += 1) {
      const previous = shifted[index - 1];
      const current = shifted[index];
      if (selectedIds.has(current) && !selectedIds.has(previous)) {
        shifted[index - 1] = current;
        shifted[index] = previous;
      }
    }
    applyNodeOrderByIds(shifted);
  }

  function closeMenu(): void {
    activeMenu = null;
  }

  function isViewEditorPath(pathname: string): boolean {
    return /^\/views\/[^/]+$/.test(pathname);
  }

  function closeLeaveViewDialog(force = false): void {
    if (leaveViewSubmitting && !force) {
      return;
    }
    leaveViewDialogOpen = false;
    pendingNavigationIntent = null;
  }

  async function confirmLeaveViewDialog(): Promise<void> {
    if (!pendingNavigationIntent) {
      return;
    }

    const intent = pendingNavigationIntent;
    leaveViewSubmitting = true;
    leaveViewDialogOpen = false;
    pendingNavigationIntent = null;
    canvasMode = "edit";
    allowConfirmedLeaveNavigation = true;

    try {
      await goto(intent.href, {
        replaceState: intent.replaceState,
      });
    } catch {
      allowConfirmedLeaveNavigation = false;
      snackbarStore.error("Unable to leave this view right now.");
    } finally {
      leaveViewSubmitting = false;
    }
  }

  const selectedGraphNode = $derived(
    graphNodes.find((node) => Boolean(node.selected)) ?? null,
  );
  const selectedGraphEdge = $derived(
    graphEdges.find((edge) => Boolean(edge.selected)) ?? null,
  );
  const selectedGraphEdgeId = $derived(selectedGraphEdge?.id ?? "");
  const selectedGraphEdgeStyle = $derived<ConnectorStyle | null>(
    selectedGraphEdge ? resolveConnectorStyle(selectedGraphEdge) : null,
  );
  const selectedGraphEdgeArrow = $derived(
    selectedGraphEdge ? resolveArrowConnectorFromEdge(selectedGraphEdge) : null,
  );
  const selectedGraphEdgePipe = $derived(
    selectedGraphEdge ? resolvePipeConnectorFromEdge(selectedGraphEdge) : null,
  );
  const hasSelectedGraphNodes = $derived(
    graphNodes.some((node) => Boolean(node.selected)),
  );
  const canOrderSelectedGraphNodes = $derived(
    rightPaneMode === "view-editor" &&
      canvasMode === "edit" &&
      hasSelectedGraphNodes,
  );
  const selectedGraphNodeData = $derived<PlantAssetNodeData | null>(
    selectedGraphNode ? (selectedGraphNode.data as PlantAssetNodeData) : null,
  );
  const selectedGraphNodeId = $derived(selectedGraphNode?.id ?? "");
  const selectedGraphNodeWidth = $derived(
    typeof selectedGraphNode?.width === "number" ? selectedGraphNode.width : null,
  );
  const selectedGraphNodeHeight = $derived(
    typeof selectedGraphNode?.height === "number" ? selectedGraphNode.height : null,
  );
  const selectedGraphNodePortOffsets = $derived(
    selectedGraphNodeData
      ? normalizeNodePortOffsets(selectedGraphNodeData.portOffsets)
      : null,
  );
  const selectedGraphWidgetDefinition = $derived(
    selectedGraphNodeData
      ? resolveAssetDefinition(selectedGraphNodeData.assetKind)
      : null,
  );

  afterNavigate(() => {
    allowConfirmedLeaveNavigation = false;
    bypassLeaveViewGuard = false;
  });

  beforeNavigate(({ from, to, cancel, type }) => {
    if (allowConfirmedLeaveNavigation || bypassLeaveViewGuard) {
      return;
    }
    if (!from || !to) {
      return;
    }
    if (!isViewEditorPath(from.url.pathname)) {
      return;
    }
    if (from.url.pathname === to.url.pathname) {
      return;
    }

    cancel();
    pendingNavigationIntent = {
      href: `${to.url.pathname}${to.url.search}${to.url.hash}`,
      replaceState: type === "popstate",
    };
    leaveViewDialogOpen = true;
  });

  onMount(() => {
    realtimeProvider.start();
  });

  onDestroy(() => {
    if (viewsSearchDebounceTimer) {
      clearTimeout(viewsSearchDebounceTimer);
    }
    if (graphHistorySnapshotTimer) {
      clearTimeout(graphHistorySnapshotTimer);
      graphHistorySnapshotTimer = null;
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
    const normalized = normalizeConnectorEdges(graphEdges);
    if (normalized.changed) {
      graphEdges = normalized.edges;
    }
  });

  $effect(() => {
    if (!routeViewId) {
      clearWidgetHandlers();
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
    if (loadedViewId !== graphHistoryViewId) {
      graphHistoryViewId = loadedViewId;
      nodeDragInProgress = false;
      resetGraphHistory();
      return;
    }
    if (
      !canEditGraphFromKeyboard() ||
      !loadedViewId ||
      applyingGraphHistory
    ) {
      return;
    }

    const snapshot = createGraphHistorySnapshot(graphNodes, graphEdges);
    const signature = snapshotSignature(snapshot);
    if (signature === graphHistorySignature) {
      return;
    }
    if (nodeDragInProgress) {
      return;
    }
    queueGraphHistorySnapshot(160);
  });

  $effect(() => {
    const nextSymbolIds = new SvelteSet<string>();
    for (const node of graphNodes) {
      if (node.type !== "plantAsset") {
        continue;
      }
      const symbolId = (node.data as PlantAssetNodeData).symbolId ?? node.id;
      nextSymbolIds.add(symbolId);
    }
    for (const symbolId of knownWidgetHandlerSymbolIds) {
      if (!nextSymbolIds.has(symbolId)) {
        unregisterWidgetHandlers(symbolId);
      }
    }
    knownWidgetHandlerSymbolIds = nextSymbolIds;
  });

  $effect(() => {
    realtimeProvider.setActive(
      rightPaneMode === "view-editor" && canvasMode === "play",
    );
  });

  $effect(() => {
    const readOnly = !canDropToCanvas;
    const interactionMode =
      rightPaneMode === "view-editor" && canvasMode === "play"
        ? "runtime"
        : "editor";
    let changed = false;
    const nextNodes = graphNodes.map((node) => {
      if (node.type !== "plantAsset") {
        return node;
      }
      const data = node.data as PlantAssetNodeData;
      if (
        data.graphReadOnly === readOnly &&
        data.interactionMode === interactionMode &&
        data.connectDraftActive === connectDragActive
      ) {
        return node;
      }
      changed = true;
      return {
        ...node,
        data: {
          ...data,
          graphReadOnly: readOnly,
          interactionMode,
          connectDraftActive: connectDragActive,
        },
      };
    });

    if (changed) {
      graphNodes = nextNodes;
    }
  });

  const subscribedTagIds = $derived(getTrackedTagIds(graphNodes));
  const liveDependencyIndex = $derived(buildGraphLiveDependencyIndex(graphNodes));

  $effect(() => {
    realtimeProvider.setDesiredIds(subscribedTagIds);
  });

  $effect(() => {
    if (rightPaneMode !== "view-editor" || canvasMode !== "play") return;
    const currentValues = $tagValues;
    const changedTagIds: string[] = [];
    const seenChangedTagIds: Record<string, true> = {};

    for (const [tagId, value] of Object.entries(currentValues)) {
      if (previousTagValues[tagId] !== value && !seenChangedTagIds[tagId]) {
        seenChangedTagIds[tagId] = true;
        changedTagIds.push(tagId);
      }
    }
    for (const tagId of Object.keys(previousTagValues)) {
      if (!(tagId in currentValues) && !seenChangedTagIds[tagId]) {
        seenChangedTagIds[tagId] = true;
        changedTagIds.push(tagId);
      }
    }
    if (changedTagIds.length === 0) {
      return;
    }

    const impactedIndexes: number[] = [];
    const seenImpactedIndexes: Record<number, true> = {};
    for (const tagId of changedTagIds) {
      const indexes = liveDependencyIndex.tagToNodeIndexes.get(tagId);
      if (!indexes) {
        continue;
      }
      for (const index of indexes) {
        if (!seenImpactedIndexes[index]) {
          seenImpactedIndexes[index] = true;
          impactedIndexes.push(index);
        }
      }
    }

    const result =
      impactedIndexes.length === graphNodes.length
        ? applyLiveValuesToGraphNodes(graphNodes, currentValues)
        : applyLiveValuesToGraphNodesAtIndexes(
            graphNodes,
            currentValues,
            impactedIndexes,
          );
    if (result.changed) {
      graphNodes = result.nodes;
    }
    previousTagValues = { ...currentValues };
  });

</script>

<svelte:window onkeydown={handleGraphShortcuts} />

<main class="flex h-dvh w-full flex-col gap-3 overflow-hidden bg-background p-4">
  <PageToolbar
    theme={$theme ?? "light"}
    {workspaceMode}
    authEnabled={data.authEnabled ?? true}
    onSelectWorkspaceMode={(mode) => void navigateWorkspaceMode(mode)}
    onToggleTheme={toggleTheme}
    {username}
  />

  <div class="flex min-h-0 flex-1 gap-4">
    <section class="flex h-full min-h-0 w-[30%] min-w-[360px] flex-col gap-2">
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
      <div class="min-h-0 flex-1">
        <VariableTree
          bind:this={variableTreeRef}
          onNodeContextMenu={handleNodeContextMenu}
          onNodeDragStart={handleNodeDragStart}
          onNodeDragEnd={handleNodeDragEnd}
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
          onTreeStateSnapshot={handleTreeStateSnapshot}
          onVarMetaChanged={applyVarMetaChangedToGraphNodes}
        />
      </div>
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
            canOrderSelection={canOrderSelectedGraphNodes}
            onBackToViewsList={() => goto("/views")}
            onToggleCanvasMode={toggleCanvasMode}
            onSave={() => void saveActiveViewCanvas()}
            onSetEntryPoint={() => void setActiveViewAsEntryPoint()}
            onSendSelectionToBack={() => reorderSelectedGraphNodes("back")}
            onSendSelectionBackward={() => reorderSelectedGraphNodes("backward")}
            onBringSelectionForward={() => reorderSelectedGraphNodes("forward")}
            onBringSelectionToFront={() => reorderSelectedGraphNodes("front")}
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
            <div
              bind:this={graphHostRef}
              class="h-full flex-1"
              role="region"
              aria-label="Graph canvas"
              oncontextmenu={handleCanvasContextMenu}
            >
              {#key `${loadedViewId ?? routeViewId ?? "view-editor"}-${graphViewportApplyNonce}`}
                <SvelteFlow
                  bind:nodes={graphNodes}
                  bind:edges={graphEdges}
                  {nodeTypes}
                  {edgeTypes}
                  initialViewport={graphViewport}
                  minZoom={0.4}
                  maxZoom={1.6}
                  zoomOnDoubleClick={false}
                  colorMode={$theme ?? "light"}
                  class={`h-full w-full rounded-md ${connectDragActive ? "is-connecting" : ""}`}
                  nodesDraggable={canDropToCanvas}
                  elementsSelectable={canDropToCanvas}
                  nodesConnectable={canDropToCanvas}
                  selectionOnDrag={canDropToCanvas}
                  panOnDrag={[1]}
                  connectionLineType={ConnectionLineType.Step}
                  connectionLineStyle={PIPE_EDGE_STYLE}
                  defaultEdgeOptions={{
                    type: "connector-arrow",
                    animated: false,
                  }}
                  proOptions={{ hideAttribution: true }}
                  zIndexMode="manual"
                  elevateNodesOnSelect={false}
                  elevateEdgesOnSelect={false}
                  onmove={handleFlowMove}
                  onnodedragstart={handleCanvasNodeDragStart}
                  onnodedragstop={handleCanvasNodeDragStop}
                  onconnect={handleConnect}
                  onconnectstart={(_event, params: OnConnectStartParams) => {
                    connectStartNodeId = params.nodeId;
                    connectDragActive = true;
                  }}
                  onconnectend={() => {
                    connectStartNodeId = null;
                    connectDragActive = false;
                  }}
                >
                  <Controls showLock={false} />
                  <MiniMap />
                  <Background />
                </SvelteFlow>
              {/key}
            </div>

            <GraphInspector
              visible={inspectorDockVisible}
              selectedNodeId={selectedGraphNodeId}
              selectedNodeData={selectedGraphNodeData}
              selectedEdgeId={selectedGraphEdgeId}
              selectedEdgeStyle={selectedGraphEdgeStyle}
              selectedEdgeArrow={selectedGraphEdgeArrow}
              selectedEdgePipe={selectedGraphEdgePipe}
              selectedWidgetDefinition={selectedGraphWidgetDefinition}
              selectedNodeWidth={selectedGraphNodeWidth}
              selectedNodeHeight={selectedGraphNodeHeight}
              selectedNodePortOffsets={selectedGraphNodePortOffsets}
              onShow={() => (inspectorDockVisible = true)}
              onHide={() => (inspectorDockVisible = false)}
              onLabelChange={(label) =>
                updateNodeData(selectedGraphNodeId, (current) => ({
                  ...current,
                  title: label.slice(0, 80),
                }))}
              onDimensionsChange={updateNodeDimensions}
              onPortOffsetsChange={updateNodePortOffsets}
              onBindingDrop={handleBindingDrop}
              onRemoveTagFromBinding={removeTagFromBinding}
              onEventBindingsChange={updateNodeEventBindings}
              onWidgetConfigChange={updateNodeWidgetConfig}
              onConnectorStyleChange={updateEdgeConnectorStyle}
              onArrowConnectorConfigChange={updateEdgeArrowConnectorConfig}
              onPipeConnectorConfigChange={updateEdgePipeConnectorConfig}
            />
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

  <WorkspaceConfirmDialogs
    {removeViewDialogOpen}
    {removeViewTarget}
    {removeViewSubmitting}
    {removeViewError}
    onRemoveViewDialogOpenChange={(open) => {
      removeViewDialogOpen = open;
    }}
    onConfirmRemoveView={() => void confirmRemoveView()}
    onCloseRemoveViewDialog={closeRemoveViewDialog}
    {removeDialogOpen}
    {removeTargetNode}
    {removeSubmitting}
    {removeError}
    wsStatus={$wsStatus}
    onRemoveDialogOpenChange={(open) => {
      removeDialogOpen = open;
    }}
    onConfirmRemoveNode={() => void confirmRemoveTargetNode()}
    onCloseRemoveDialog={closeRemoveDialog}
    {removeMultipleDialogOpen}
    {removeMultipleSubmitting}
    {removeMultipleError}
    onRemoveMultipleDialogOpenChange={(open) => {
      removeMultipleDialogOpen = open;
    }}
    onConfirmRemoveMultiple={() => void confirmRemoveMultiple()}
    onCloseRemoveMultipleDialog={closeRemoveMultipleDialog}
    {leaveViewDialogOpen}
    activeViewName={activeView?.name ?? null}
    {routeViewId}
    {leaveViewSubmitting}
    canConfirmLeave={Boolean(pendingNavigationIntent)}
    onLeaveViewDialogOpenChange={(open) => {
      leaveViewDialogOpen = open;
    }}
    onConfirmLeaveView={() => void confirmLeaveViewDialog()}
    onCloseLeaveViewDialog={closeLeaveViewDialog}
  />

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
