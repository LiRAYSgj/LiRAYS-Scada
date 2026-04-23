<script lang="ts">
  import { beforeNavigate, goto } from "$app/navigation";
  import { page } from "$app/stores";
  import { Button } from "$lib/components/Button";
  import PlantAssetNode from "$lib/features/graph/components/PlantAssetNode.svelte";
  import ArrowConnectorEdge from "$lib/features/graph/components/ArrowConnectorEdge.svelte";
  import PipeConnectorEdge from "$lib/features/graph/components/PipeConnectorEdge.svelte";
  import { resolveAssetDefinition } from "$lib/features/graph/assets/registry";
  import { normalizeNodePortOffsets } from "$lib/features/graph/assets/types";
  import type {
    ContextMenuActionItem,
    PlantAssetNodeData,
    WidgetEventAction,
    WidgetEventBinding,
    WidgetInteractionEventName,
  } from "$lib/features/graph/assets/types";
  import {
    applyLiveValuesToGraphNodesAtIndexes,
    applyLiveValuesToGraphNodes,
    buildGraphLiveDependencyIndex,
    getTrackedTagIds,
  } from "$lib/features/graph/live-utils";
  import {
    DEFAULT_PIPE_CONNECTION_LINE_STYLE,
    normalizeConnectorEdges,
  } from "$lib/features/graph/connectors";
  import {
    clearWidgetHandlers,
    registerWidgetHandlers,
  } from "$lib/features/graph/widget-handlers";
  import { createPageTagRealtimeProvider } from "$lib/features/realtime/page-tag-realtime-provider";
  import { getView } from "$lib/features/views/api/views-api";
  import { deserializeCanvasState } from "$lib/features/views/types";
  import { resolveTagStreamWsEndpoint } from "$lib/core/ws/resolve-ws-endpoint";
  import { tagStreamClient } from "$lib/core/ws/tag-stream-client";
  import { themeStore } from "$lib/stores/theme";
  import { snackbarStore } from "$lib/stores/snackbar";
  import {
    Background,
    ConnectionLineType,
    getViewportForBounds,
    SvelteFlow,
    type Edge,
    type Node,
    type Viewport,
  } from "@xyflow/svelte";
  import "@xyflow/svelte/dist/style.css";
  import { SvelteMap, SvelteSet } from "svelte/reactivity";
  import { onDestroy, onMount } from "svelte";
  import type { TagScalarValue } from "$lib/core/ws/types";
  import type { VarMetaChanged } from "@lirays/scada-proto";

  const WS_ENDPOINT = resolveTagStreamWsEndpoint();
  const PIPE_EDGE_STYLE = DEFAULT_PIPE_CONNECTION_LINE_STYLE;

  const theme = themeStore;
  const realtimeProvider = createPageTagRealtimeProvider(WS_ENDPOINT);
  const tagValues = realtimeProvider.values;

  const nodeTypes = {
    plantAsset: PlantAssetNode,
  };
  const edgeTypes = {
    "connector-arrow": ArrowConnectorEdge,
    "connector-pipe": PipeConnectorEdge,
  };

  const routeViewId = $derived($page.params.id ?? null);

  let graphNodes = $state.raw<Node[]>([]);
  let graphEdges = $state.raw<Edge[]>([]);
  let graphViewport = $state<Viewport>({ x: 0, y: 0, zoom: 1 });
  let graphHostRef = $state<HTMLElement | null>(null);
  let pendingViewportFit = $state(false);
  let loading = $state(false);
  let loadError = $state("");
  let runtimeContextMenu = $state<{
    x: number;
    y: number;
    items: ContextMenuActionItem[];
  } | null>(null);
  let requestCounter = 0;
  let unsubscribeTreeChanges: (() => void) | null = null;
  let previousTagValues = $state.raw<Record<string, TagScalarValue>>({});
  let graphViewportApplyNonce = $state(0);

  onMount(() => {
    realtimeProvider.start();
    realtimeProvider.setActive(true);
    unsubscribeTreeChanges = tagStreamClient.treeChanges.subscribe((ev) => {
      if (!ev?.varMetaChangedEvent?.length) {
        return;
      }
      applyVarMetaChangedToGraphNodes(ev.varMetaChangedEvent);
    });
  });

  beforeNavigate(({ to }) => {
    // Clear current subscriptions before route changes so unsubscribe happens
    // while the current socket is still active.
    realtimeProvider.setDesiredIds([]);

    const nextPath = to?.url.pathname ?? "";
    if (!nextPath.startsWith("/runtime")) {
      realtimeProvider.setActive(false);
    }
  });

  onDestroy(() => {
    unsubscribeTreeChanges?.();
    unsubscribeTreeChanges = null;
    clearWidgetHandlers();
    realtimeProvider.stop();
  });

  function applyVarMetaChangedToGraphNodes(changes: VarMetaChanged[]): void {
    if (changes.length === 0 || graphNodes.length === 0) {
      return;
    }

    const byVarId = new SvelteMap<string, VarMetaChanged>();
    for (const change of changes) {
      byVarId.set(change.varId, change);
    }

    let changed = false;
    const nextNodes = graphNodes.map((node) => {
      if (node.type !== "plantAsset") {
        return node;
      }
      const data = node.data as PlantAssetNodeData;
      let nodeChanged = false;

      let nextBindings: PlantAssetNodeData["bindings"] = data.bindings;
      if (data.bindings) {
        const updatedBindings = Object.fromEntries(
          Object.entries(data.bindings).map(([bindingKey, tags]) => {
            const updatedTags = tags.map((tag) => {
              if (tag.kind !== "tag") {
                return tag;
              }
              const change = byVarId.get(tag.id);
              if (!change) {
                return tag;
              }
              const nextMin = change.min ?? undefined;
              const nextMax = change.max ?? undefined;
              if (tag.min === nextMin && tag.max === nextMax) {
                return tag;
              }
              nodeChanged = true;
              return {
                ...tag,
                min: nextMin,
                max: nextMax,
              };
            });
            return [bindingKey, updatedTags];
          }),
        );
        if (nodeChanged) {
          nextBindings = updatedBindings;
        }
      }

      const source = data.sourceNode;
      const sourceChange =
        source?.kind === "tag" ? byVarId.get(source.id) : undefined;
      let nextSource = source;
      if (sourceChange) {
        const nextMin = sourceChange.min ?? undefined;
        const nextMax = sourceChange.max ?? undefined;
        if (source.min !== nextMin || source.max !== nextMax) {
          nextSource = {
            ...source,
            min: nextMin,
            max: nextMax,
          };
          nodeChanged = true;
        }
      }

      if (!nodeChanged) {
        return node;
      }

      changed = true;
      return {
        ...node,
        data: {
          ...data,
          bindings: nextBindings,
          sourceNode: nextSource,
        },
      };
    });

    if (changed) {
      graphNodes = nextNodes;
    }
  }

  interface BoundTagMetadata {
    min?: number;
    max?: number;
    unit?: string;
    dataType?: string;
    options?: string[];
  }

  function getParentIdFromNodePath(pathOrId: string): string | null {
    if (!pathOrId || pathOrId === "/") {
      return null;
    }
    const trimmed = pathOrId.endsWith("/") ? pathOrId.slice(0, -1) : pathOrId;
    const idx = trimmed.lastIndexOf("/");
    if (idx <= 0) {
      return null;
    }
    return trimmed.slice(0, idx);
  }

  function isStringArrayEqual(a?: string[], b?: string[]): boolean {
    if (a === b) {
      return true;
    }
    const left = a ?? [];
    const right = b ?? [];
    if (left.length !== right.length) {
      return false;
    }
    for (let index = 0; index < left.length; index += 1) {
      if (left[index] !== right[index]) {
        return false;
      }
    }
    return true;
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

  function applyBoundMetadataToNodes(
    nodes: Node[],
    byTagId: Map<string, BoundTagMetadata>,
  ): Node[] {
    if (byTagId.size === 0 || nodes.length === 0) {
      return nodes;
    }

    let changed = false;
    const nextNodes = nodes.map((node) => {
      if (node.type !== "plantAsset") {
        return node;
      }
      const data = node.data as PlantAssetNodeData;
      let nodeChanged = false;

      let nextBindings: PlantAssetNodeData["bindings"] = data.bindings;
      if (data.bindings) {
        const updatedBindings = Object.fromEntries(
          Object.entries(data.bindings).map(([bindingKey, tags]) => {
            const updatedTags = tags.map((tag) => {
              if (tag.kind !== "tag") {
                return tag;
              }
              const latest = byTagId.get(tag.id);
              if (!latest) {
                return tag;
              }
              const sameOptions = isStringArrayEqual(tag.options, latest.options);
              if (
                tag.min === latest.min &&
                tag.max === latest.max &&
                tag.unit === latest.unit &&
                sameOptions
              ) {
                return tag;
              }
              nodeChanged = true;
              return {
                ...tag,
                min: latest.min,
                max: latest.max,
                unit: latest.unit,
                options: latest.options,
              };
            });
            return [bindingKey, updatedTags];
          }),
        );
        if (nodeChanged) {
          nextBindings = updatedBindings;
        }
      }

      const source = data.sourceNode;
      let nextSource = source;
      if (source?.kind === "tag") {
        const latest = byTagId.get(source.id);
        if (latest) {
          const sameOptions = isStringArrayEqual(source.options, latest.options);
          if (
            source.min !== latest.min ||
            source.max !== latest.max ||
            source.unit !== latest.unit ||
            !sameOptions
          ) {
            nextSource = {
              ...source,
              min: latest.min,
              max: latest.max,
              unit: latest.unit,
              options: latest.options,
            };
            nodeChanged = true;
          }
        }
      }

      if (!nodeChanged) {
        return node;
      }

      changed = true;
      return {
        ...node,
        data: {
          ...data,
          bindings: nextBindings,
          sourceNode: nextSource,
        },
      };
    });

    return changed ? nextNodes : nodes;
  }

  $effect(() => {
    const normalized = normalizeConnectorEdges(graphEdges);
    if (normalized.changed) {
      graphEdges = normalized.edges;
    }
  });

  $effect(() => {
    const viewId = routeViewId;
    if (!viewId) {
      previousTagValues = {};
      return;
    }

    void loadRuntimeView(viewId);
  });

  const liveDependencyIndex = $derived(buildGraphLiveDependencyIndex(graphNodes));

  $effect(() => {
    const currentValues = $tagValues;
    if (graphNodes.length === 0) {
      previousTagValues = { ...currentValues };
      return;
    }

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

  $effect(() => {
    if (!pendingViewportFit || loading || !graphHostRef) {
      return;
    }

    const nextViewport = resolveRuntimeFitViewport(graphNodes);
    if (!isSameViewport(graphViewport, nextViewport)) {
      graphViewport = nextViewport;
      graphViewportApplyNonce += 1;
    }
    pendingViewportFit = false;
  });

  async function loadRuntimeView(viewId: string): Promise<void> {
    const requestId = ++requestCounter;
    loading = true;
    loadError = "";
    // Route changed: clear old subscriptions until the new view graph is loaded.
    realtimeProvider.setDesiredIds([]);
    clearWidgetHandlers();

    try {
      const view = await getView(viewId);
      if (requestId !== requestCounter) {
        return;
      }

      const canvas = deserializeCanvasState(view.canvas_json);
      const normalizedEdges = normalizeConnectorEdges(canvas.edges);
      const hydratedNodes = hydrateCanvasNodes(canvas.nodes);
      const hydratedEdges = hydrateCanvasEdges(normalizedEdges.edges);
      const trackedTagIds = getTrackedTagIds(hydratedNodes);
      const metadataByTagId = await fetchLatestMetadataForTagIds(trackedTagIds);
      if (requestId !== requestCounter) {
        return;
      }
      const hydratedWithMetadata = applyBoundMetadataToNodes(
        hydratedNodes,
        metadataByTagId,
      );
      graphNodes = hydratedWithMetadata;
      graphEdges = hydratedEdges;
      graphViewport = canvas.viewport;
      graphViewportApplyNonce += 1;
      pendingViewportFit = true;
      previousTagValues = {};
      realtimeProvider.setDesiredIds(getTrackedTagIds(hydratedWithMetadata));
    } catch {
      if (requestId !== requestCounter) {
        return;
      }

      graphNodes = [];
      graphEdges = [];
      graphViewport = { x: 0, y: 0, zoom: 1 };
      graphViewportApplyNonce += 1;
      pendingViewportFit = false;
      previousTagValues = {};
      loadError = "Unable to load runtime view.";
      realtimeProvider.setDesiredIds([]);
    } finally {
      if (requestId === requestCounter) {
        loading = false;
      }
    }
  }

  function getNodeDataById(nodeId: string): PlantAssetNodeData | null {
    const match = graphNodes.find((node) => node.id === nodeId);
    if (!match) return null;
    return match.data as PlantAssetNodeData;
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

  function hydratePlantAssetNodeData(
    nodeId: string,
    raw: PlantAssetNodeData,
  ): PlantAssetNodeData {
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
      onWidgetEvent: (
        eventName: WidgetInteractionEventName,
        payload?: unknown,
        event?: MouseEvent,
      ) => void handleWidgetEvent(nodeId, eventName, payload, event),
    });

    const widgetConfig = {
      ...(definition.defaultConfig ?? {}),
      ...(raw.widgetConfig ?? {}),
    };

    return {
      ...raw,
      symbolId,
      graphReadOnly: true,
      primaryBindingKey,
      interactionMode: "runtime",
      eventBindings: sanitizeEventBindings(raw.assetKind, raw.eventBindings ?? []),
      widgetConfig,
      portOffsets: normalizeNodePortOffsets(raw.portOffsets),
    };
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

  function hydrateCanvasNodes(nodes: Node[]): Node[] {
    return nodes.map((node) => {
      if (node.type !== "plantAsset") {
        return {
          ...node,
          selected: false,
        };
      }

      const data = node.data as PlantAssetNodeData;
      return {
        ...node,
        selected: false,
        data: hydratePlantAssetNodeData(node.id, data),
      };
    });
  }

  function hydrateCanvasEdges(edges: Edge[]): Edge[] {
    return edges.map((edge) => ({
      ...edge,
      selected: false,
    }));
  }

  function computeNodeBounds(nodes: Node[]): {
    x: number;
    y: number;
    width: number;
    height: number;
  } | null {
    if (nodes.length === 0) {
      return null;
    }

    let minX = Number.POSITIVE_INFINITY;
    let minY = Number.POSITIVE_INFINITY;
    let maxX = Number.NEGATIVE_INFINITY;
    let maxY = Number.NEGATIVE_INFINITY;

    for (const node of nodes) {
      const width = typeof node.width === "number" ? node.width : 240;
      const height = typeof node.height === "number" ? node.height : 160;
      minX = Math.min(minX, node.position.x);
      minY = Math.min(minY, node.position.y);
      maxX = Math.max(maxX, node.position.x + width);
      maxY = Math.max(maxY, node.position.y + height);
    }

    if (!Number.isFinite(minX) || !Number.isFinite(minY)) {
      return null;
    }

    return {
      x: minX,
      y: minY,
      width: Math.max(1, maxX - minX),
      height: Math.max(1, maxY - minY),
    };
  }

  function resolveRuntimeFitViewport(nodes: Node[]): Viewport {
    const bounds = computeNodeBounds(nodes);
    if (!bounds || !graphHostRef) {
      return { x: 0, y: 0, zoom: 1 };
    }

    const canvasWidth = Math.max(1, graphHostRef.clientWidth);
    const canvasHeight = Math.max(1, graphHostRef.clientHeight);
    return getViewportForBounds(bounds, canvasWidth, canvasHeight, 0.4, 1.6, 0.18);
  }

  function isSameViewport(a: Viewport, b: Viewport): boolean {
    const epsilon = 0.01;
    return (
      Math.abs(a.x - b.x) < epsilon &&
      Math.abs(a.y - b.y) < epsilon &&
      Math.abs(a.zoom - b.zoom) < epsilon
    );
  }

  function handleWidgetEvent(
    nodeId: string,
    eventName: WidgetInteractionEventName,
    _payload?: unknown,
    event?: MouseEvent,
  ): void {
    const data = getNodeDataById(nodeId);
    if (!data) {
      return;
    }
    const supportedEvents = resolveAssetDefinition(data.assetKind).supportedEvents ?? [];
    if (!supportedEvents.includes(eventName)) {
      return;
    }

    const binding = (data.eventBindings ?? []).find((item) => item.on === eventName);
    if (!binding) {
      return;
    }

    for (const action of binding.do) {
      void executeWidgetAction(action, event);
    }
  }

  async function executeWidgetAction(
    action: WidgetEventAction,
    event?: MouseEvent,
  ): Promise<void> {
    if (action.type === "navigateRuntimeView") {
      const viewId = action.params.viewId?.trim();
      if (!viewId) {
        return;
      }

      runtimeContextMenu = null;
      await goto(`/runtime/${viewId}`);
      return;
    }

    if (action.type === "openContextMenu") {
      if (action.params.items.length === 0) {
        runtimeContextMenu = null;
        return;
      }

      runtimeContextMenu = {
        x: event?.clientX ?? 32,
        y: event?.clientY ?? 32,
        items: action.params.items,
      };
    }
  }

  async function onRuntimeMenuItemClick(item: ContextMenuActionItem): Promise<void> {
    runtimeContextMenu = null;
    const viewId = item.viewId?.trim();
    if (!viewId) {
      return;
    }

    await goto(`/runtime/${viewId}`);
  }
</script>

{#if loadError}
  <div class="flex h-full items-center justify-center px-6 py-10">
    <div class="max-w-xl px-6 py-5 text-center">
      <h2 class="text-base font-semibold text-foreground">Failed to load view</h2>
      <p class="mt-2 text-sm text-muted-foreground">{loadError}</p>
      <div class="mt-4 flex justify-center">
        <Button
          variant="outline-accent"
          label="Retry"
          disabled={!routeViewId || loading}
          onclick={() => {
            if (routeViewId) {
              void loadRuntimeView(routeViewId);
            }
          }}
        />
      </div>
    </div>
  </div>
{:else if loading}
  <div class="flex h-full items-center justify-center text-sm text-muted-foreground">
    Loading runtime view...
  </div>
{:else}
  <div bind:this={graphHostRef} class="h-full w-full">
    {#key `${routeViewId ?? "runtime"}-${graphViewportApplyNonce}`}
      <SvelteFlow
        bind:nodes={graphNodes}
        bind:edges={graphEdges}
        {nodeTypes}
        {edgeTypes}
        initialViewport={graphViewport}
        minZoom={0.4}
        maxZoom={1.6}
        zoomOnDoubleClick={false}
        zoomOnScroll={false}
        zoomOnPinch={false}
        panOnDrag={false}
        panOnScroll={false}
        colorMode={$theme ?? "light"}
        class="h-full w-full"
        nodesDraggable={false}
        elementsSelectable={false}
        nodesConnectable={false}
        selectionOnDrag={false}
        connectionLineType={ConnectionLineType.Step}
        connectionLineStyle={PIPE_EDGE_STYLE}
        defaultEdgeOptions={{
          type: "connector-arrow",
          animated: false,
        }}
        proOptions={{ hideAttribution: true }}
      >
        <Background />
      </SvelteFlow>
    {/key}
  </div>
{/if}

{#if runtimeContextMenu}
  <button
    type="button"
    class="fixed inset-0 z-40 bg-transparent"
    aria-label="Close context menu"
    onclick={() => {
      runtimeContextMenu = null;
    }}
  ></button>

  <div
    class="fixed z-50 min-w-[220px] rounded-md border border-border bg-card p-1 shadow-lg"
    style={`left:${runtimeContextMenu.x}px;top:${runtimeContextMenu.y}px;`}
  >
    {#each runtimeContextMenu.items as item (item.id)}
      <button
        type="button"
        class="block w-full rounded px-2 py-1.5 text-left text-xs text-foreground hover:bg-muted disabled:opacity-50"
        disabled={item.enabled === false}
        onclick={() => void onRuntimeMenuItemClick(item)}
      >
        {item.label}
      </button>
    {/each}
  </div>
{/if}
