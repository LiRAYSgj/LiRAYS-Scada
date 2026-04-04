<script lang="ts">
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
  import { Layers, Plus, Trash2, Pencil } from "lucide-svelte";
  import { sanitizeIdentifierLike, sanitizeText } from "$lib/forms/sanitize";

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
  let canvasMode = $state<CanvasMode>("edit");
  let graphNodes = $state<Node[]>([]);
  let graphEdges = $state<Edge[]>([]);
  let graphHostRef: HTMLElement | null = null;
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
      if (browser) {
        // Debug payload produced by Namespace Builder right before backend save.
        /* console.log(
          "[NamespaceBuilder] JSON before addBulkNamespace:",
          JSON.stringify(json, null, 2),
        ); */
      }
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

  $effect(() => {
    realtimeProvider.setActive(canvasMode === "play");
  });

  const subscribedTagIds = $derived(getTrackedTagIds(graphNodes));

  $effect(() => {
    realtimeProvider.setDesiredIds(subscribedTagIds);
  });

  $effect(() => {
    if (canvasMode !== "play") return;
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
    {canvasMode}
    onToggleCanvasMode={toggleCanvasMode}
    onToggleTheme={toggleTheme}
    onOpenAddDialog={openTreeAddDialog}
    onOpenNamespaceBuilder={openNamespaceBuilderFromToolbar}
    isAddDisabled={false}
    {username}
    {multiSelectMode}
    onToggleMultiSelect={() => {
      multiSelectMode = !multiSelectMode;
      if (!multiSelectMode) treeSelection = new Set();
    }}
    selectionCount={treeSelection.size}
    onRemoveSelection={openRemoveMultipleDialog}
    onSelectAll={selectAllSelection}
  />

  <div class="flex min-h-0 flex-1 gap-4">
    <section class="h-full w-[30%] min-w-[360px]">
      <VariableTree
        onNodeContextMenu={handleNodeContextMenu}
        onNodeDragStart={handleNodeDragStart}
        onNodeDragEnd={handleNodeDragEnd}
        onRootId={(id) => (treeRootId = id)}
        onCreateItem={createTreeItem}
        onEditMeta={editTreeMeta}
        websocketStatus={$wsStatus}
        realtimeEnabled={canvasMode === "play"}
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
      <div class="relative flex h-full w-full">
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

                <div
                  class="rounded border border-border p-2"
                >
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
                    <div
                      class="rounded border border-border p-2"
                    >
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
      <!-- <div class="pointer-events-none absolute left-3 top-2 text-[11px] text-muted-foreground">
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
