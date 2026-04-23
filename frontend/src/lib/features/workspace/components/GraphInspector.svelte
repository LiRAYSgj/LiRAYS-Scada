<script lang="ts">
  import { onMount } from "svelte";
  import { slide } from "svelte/transition";
  import { Trash2 } from "lucide-svelte";
  import { Button } from "$lib/components/Button";
  import { Input } from "$lib/components/ui/input";
  import { NumberField } from "$lib/components/ui/number-field";
  import { Textarea } from "$lib/components/ui/textarea";
  import * as Select from "$lib/components/ui/select";
  import { Autocomplete, type AutocompleteItem } from "$lib/components/ui/autocomplete";
  import {
    listViews,
  } from "$lib/features/views/api/views-api";
  import type { WidgetConfigFieldSchema, WidgetConfigThresholdRange } from "$lib/scada/plugins/types";
  import type {
    GraphAssetDefinition,
    NodePortOffsets,
    PlantAssetNodeData,
    WidgetEventAction,
    WidgetEventBinding,
    WidgetInteractionEventName,
  } from "$lib/features/graph/assets/types";
  import type {
    ArrowConnectorConfig,
    ConnectorStyle,
    PipeConnectorConfig,
  } from "$lib/features/graph/connectors";

  interface Props {
    visible: boolean;
    selectedNodeId: string;
    selectedEdgeId: string;
    selectedEdgeStyle: ConnectorStyle | null;
    selectedEdgeArrow: ArrowConnectorConfig | null;
    selectedEdgePipe: PipeConnectorConfig | null;
    selectedNodeData: PlantAssetNodeData | null;
    selectedWidgetDefinition: GraphAssetDefinition | null;
    selectedNodeWidth: number | null;
    selectedNodeHeight: number | null;
    selectedNodePortOffsets: NodePortOffsets | null;
    onShow: () => void;
    onHide: () => void;
    onLabelChange: (label: string) => void;
    onDimensionsChange: (
      nodeId: string,
      dimensions: { width?: number; height?: number },
    ) => void;
    onPortOffsetsChange: (
      nodeId: string,
      offsets: Partial<NodePortOffsets>,
    ) => void;
    onBindingDrop: (
      event: DragEvent,
      nodeId: string,
      bindingKey: string,
    ) => void;
    onRemoveTagFromBinding: (
      nodeId: string,
      bindingKey: string,
      tagId: string,
    ) => void;
    onEventBindingsChange: (
      nodeId: string,
      bindings: WidgetEventBinding[],
    ) => void;
    onWidgetConfigChange: (
      nodeId: string,
      widgetConfig: Record<string, unknown>,
    ) => void;
    onConnectorStyleChange: (edgeId: string, style: ConnectorStyle) => void;
    onArrowConnectorConfigChange: (
      edgeId: string,
      patch: Partial<ArrowConnectorConfig>,
    ) => void;
    onPipeConnectorConfigChange: (
      edgeId: string,
      patch: Partial<PipeConnectorConfig>,
    ) => void;
  }

  let {
    visible,
    selectedNodeId,
    selectedEdgeId,
    selectedEdgeStyle,
    selectedEdgeArrow,
    selectedEdgePipe,
    selectedNodeData,
    selectedWidgetDefinition,
    selectedNodeWidth,
    selectedNodeHeight,
    selectedNodePortOffsets,
    onShow,
    onHide,
    onLabelChange,
    onDimensionsChange,
    onPortOffsetsChange,
    onBindingDrop,
    onRemoveTagFromBinding,
    onEventBindingsChange,
    onWidgetConfigChange,
    onConnectorStyleChange,
    onArrowConnectorConfigChange,
    onPipeConnectorConfigChange,
  }: Props = $props();
  const MIN_PANEL_WIDTH = 300;
  const MAX_PANEL_WIDTH = 560;
  const DEFAULT_PANEL_WIDTH = 400;
  let panelWidth = $state(DEFAULT_PANEL_WIDTH);
  let resizing = false;
  let dragStartX = 0;
  let dragStartWidth = DEFAULT_PANEL_WIDTH;
  let inspectorRoot = $state<HTMLElement | null>(null);
  let suppressOutsideRelease = false;

  type ActionType = WidgetEventAction["type"];
  interface ContextMenuOptionDraft {
    id: string;
    label: string;
    viewId?: string;
    viewName?: string;
    enabled?: boolean;
  }

  const EVENT_OPTIONS: Array<{ value: WidgetInteractionEventName; label: string }> = [
    { value: "click", label: "Click" },
    { value: "doubleClick", label: "Double click" },
    { value: "rightClick", label: "Right click" },
  ];

  const ACTION_OPTIONS: Array<{ value: ActionType; label: string }> = [
    { value: "navigateRuntimeView", label: "Navigate to runtime view" },
    { value: "openContextMenu", label: "Open context menu" },
  ];
  let collapsedBindings = $state<number[]>([]);
  const DEFAULT_MIN_NODE_WIDTH = 240;
  const DEFAULT_MIN_NODE_HEIGHT = 160;
  const CONNECTOR_STYLE_OPTIONS: Array<{ value: ConnectorStyle; label: string }> = [
    { value: "arrow", label: "Arrow" },
    { value: "pipe", label: "Pipe" },
  ];

  function getMinNodeWidth(): number {
    return selectedWidgetDefinition?.minWidth ?? DEFAULT_MIN_NODE_WIDTH;
  }

  function getMinNodeHeight(): number {
    return selectedWidgetDefinition?.minHeight ?? DEFAULT_MIN_NODE_HEIGHT;
  }

  function normalizeDimension(
    value: number,
    min: number,
  ): number {
    if (!Number.isFinite(value)) {
      return min;
    }
    return Math.max(min, Math.round(value));
  }

  function effectiveNodeWidth(): number {
    const min = getMinNodeWidth();
    if (typeof selectedNodeWidth !== "number") {
      return min;
    }
    return normalizeDimension(selectedNodeWidth, min);
  }

  function effectiveNodeHeight(): number {
    const min = getMinNodeHeight();
    if (typeof selectedNodeHeight !== "number") {
      return min;
    }
    return normalizeDimension(selectedNodeHeight, min);
  }

  function activeConnectorStyle(): ConnectorStyle {
    return selectedEdgeStyle ?? "pipe";
  }

  function activeArrowConfig(): ArrowConnectorConfig {
    return (
      selectedEdgeArrow ?? {
        color: "#4d5e75",
        thickness: 3,
        arrowSize: 16,
      }
    );
  }

  function activePipeConfig(): PipeConnectorConfig {
    return (
      selectedEdgePipe ?? {
        thickness: 10,
        flangeScale: 1,
      }
    );
  }

  function activeConfigSchema(): WidgetConfigFieldSchema[] {
    return selectedWidgetDefinition?.configSchema ?? [];
  }

  function portOffset(side: keyof NodePortOffsets): number {
    return selectedNodePortOffsets?.[side] ?? 50;
  }

  function activeConfig(): Record<string, unknown> {
    return selectedNodeData?.widgetConfig ?? {};
  }

  function getConfigValue(field: WidgetConfigFieldSchema): unknown {
    const value = activeConfig()[field.key];
    if (value !== undefined) {
      return value;
    }
    return selectedWidgetDefinition?.defaultConfig?.[field.key];
  }

  function getSelectOptionLabel(
    field: WidgetConfigFieldSchema,
    value: string,
  ): string {
    const option = field.options?.find((item) => item.value === value);
    return option?.label ?? value;
  }

  function updateConfigField(key: string, value: unknown): void {
    if (!selectedNodeData) {
      return;
    }
    const next = {
      ...(selectedWidgetDefinition?.defaultConfig ?? {}),
      ...activeConfig(),
      [key]: value,
    };
    onWidgetConfigChange(selectedNodeId, next);
  }

  function readThresholdRanges(field: WidgetConfigFieldSchema): WidgetConfigThresholdRange[] {
    const raw = getConfigValue(field);
    if (!Array.isArray(raw)) {
      return [];
    }
    const ranges: WidgetConfigThresholdRange[] = [];
    for (const entry of raw) {
      if (!entry || typeof entry !== "object") {
        continue;
      }
      const source = entry as Partial<WidgetConfigThresholdRange>;
      ranges.push({
        min: typeof source.min === "number" ? source.min : undefined,
        max: typeof source.max === "number" ? source.max : undefined,
        color: typeof source.color === "string" ? source.color : "#546a0e",
        label: typeof source.label === "string" ? source.label : "",
      });
    }
    return ranges;
  }

  function addThresholdRange(field: WidgetConfigFieldSchema): void {
    const next = [
      ...readThresholdRanges(field),
      {
        min: undefined,
        max: undefined,
        color: "#546a0e",
        label: "",
      },
    ];
    updateConfigField(field.key, next);
  }

  function removeThresholdRange(
    field: WidgetConfigFieldSchema,
    index: number,
  ): void {
    const next = readThresholdRanges(field).filter((_, i) => i !== index);
    updateConfigField(field.key, next);
  }

  function updateThresholdRange(
    field: WidgetConfigFieldSchema,
    index: number,
    patch: Partial<WidgetConfigThresholdRange>,
  ): void {
    const next = readThresholdRanges(field).map((entry, i) =>
      i === index ? { ...entry, ...patch } : entry,
    );
    updateConfigField(field.key, next);
  }

  function defaultAction(): WidgetEventAction {
    return {
      type: "navigateRuntimeView",
      params: {
        viewId: "",
      },
    };
  }

  function defaultBinding(): WidgetEventBinding {
    const firstEvent = availableEventOptions()[0]?.value ?? "click";
    return {
      on: firstEvent,
      do: [defaultAction()],
    };
  }

  function availableEventOptions(): Array<{ value: WidgetInteractionEventName; label: string }> {
    const supported = selectedWidgetDefinition?.supportedEvents ?? [];
    if (supported.length === 0) {
      return [];
    }
    return EVENT_OPTIONS.filter((option) => supported.includes(option.value));
  }

  function currentBindings(): WidgetEventBinding[] {
    return selectedNodeData?.eventBindings ?? [];
  }

  function isEventAlreadyBound(
    eventName: WidgetInteractionEventName,
    exceptIndex?: number,
  ): boolean {
    return currentBindings().some(
      (binding, index) => index !== exceptIndex && binding.on === eventName,
    );
  }

  function pushBindings(next: WidgetEventBinding[]): void {
    if (!selectedNodeData) {
      return;
    }
    onEventBindingsChange(selectedNodeId, next);
  }

  function updateBinding(
    index: number,
    updater: (binding: WidgetEventBinding) => WidgetEventBinding,
  ): void {
    const bindings = currentBindings();
    if (!bindings[index]) {
      return;
    }

    pushBindings(
      bindings.map((binding, i) => (i === index ? updater(binding) : binding)),
    );
  }

  function updatePrimaryAction(
    index: number,
    updater: (action: WidgetEventAction) => WidgetEventAction,
  ): void {
    updateBinding(index, (binding) => {
      const first = binding.do[0] ?? defaultAction();
      return {
        ...binding,
        do: [updater(first)],
      };
    });
  }

  function getPrimaryAction(binding: WidgetEventBinding): WidgetEventAction {
    return binding.do[0] ?? defaultAction();
  }

  function addInteractionBinding(): void {
    const nextEvent = availableEventOptions().find(
      (option) =>
        !isEventAlreadyBound(option.value as WidgetInteractionEventName),
    )?.value;
    if (!nextEvent) {
      return;
    }

    pushBindings([
      ...currentBindings(),
      {
        ...defaultBinding(),
        on: nextEvent,
      },
    ]);
    collapsedBindings = collapsedBindings.filter((index) => index >= 0);
  }

  function removeInteractionBinding(index: number): void {
    pushBindings(currentBindings().filter((_, i) => i !== index));
    collapsedBindings = collapsedBindings
      .filter((i) => i !== index)
      .map((i) => (i > index ? i - 1 : i));
  }

  function updateEventType(index: number, value: string): void {
    const nextEvent = value as WidgetInteractionEventName;
    updateBinding(index, (binding) => ({
      ...binding,
      on: nextEvent,
    }));
  }

  function updateActionType(index: number, value: string): void {
    updatePrimaryAction(index, (action) => {
      const type = value as ActionType;
      if (type === action.type) {
        return action;
      }

      if (type === "navigateRuntimeView") {
        return {
          type,
          params: {
            viewId: "",
          },
        };
      }

      return {
        type,
        params: {
          items: [],
        },
      };
    });
  }

  function updateNavigateTarget(index: number, item: AutocompleteItem | null): void {
    updatePrimaryAction(index, (action) => {
      if (action.type !== "navigateRuntimeView") {
        return action;
      }

      return {
        ...action,
        params: {
          viewId: item?.id ?? "",
          viewName: item?.label,
        },
      };
    });
  }

  function addContextMenuItem(index: number): void {
    updatePrimaryAction(index, (action) => {
      if (action.type !== "openContextMenu") {
        return action;
      }

      const nextItem: ContextMenuOptionDraft = {
        id: `item-${Math.random().toString(36).slice(2, 9)}`,
        label: "Option",
        enabled: true,
      };

      return {
        ...action,
        params: {
          items: [...action.params.items, nextItem],
        },
      };
    });
  }

  function removeContextMenuItem(bindingIndex: number, itemIndex: number): void {
    updatePrimaryAction(bindingIndex, (action) => {
      if (action.type !== "openContextMenu") {
        return action;
      }

      return {
        ...action,
        params: {
          items: action.params.items.filter((_, index) => index !== itemIndex),
        },
      };
    });
  }

  function updateContextMenuItemLabel(
    bindingIndex: number,
    itemIndex: number,
    label: string,
  ): void {
    updatePrimaryAction(bindingIndex, (action) => {
      if (action.type !== "openContextMenu") {
        return action;
      }

      return {
        ...action,
        params: {
          items: action.params.items.map((item, index) =>
            index === itemIndex
              ? {
                  ...item,
                  label,
                }
              : item,
          ),
        },
      };
    });
  }

  function updateContextMenuItemView(
    bindingIndex: number,
    itemIndex: number,
    selected: AutocompleteItem | null,
  ): void {
    updatePrimaryAction(bindingIndex, (action) => {
      if (action.type !== "openContextMenu") {
        return action;
      }

      return {
        ...action,
        params: {
          items: action.params.items.map((item, index) =>
            index === itemIndex
              ? {
                  ...item,
                  viewId: selected?.id,
                  viewName: selected?.label,
                }
              : item,
          ),
        },
      };
    });
  }

  async function searchViews(query: string): Promise<AutocompleteItem[]> {
    const page = await listViews({
      page: 1,
      pageSize: 20,
      sortBy: "name",
      sortDirection: "asc",
      search: query,
    });

    return page.items.map((view) => ({
      id: view.id,
      label: view.name,
      description: view.is_entry_point ? "Entry point" : undefined,
    }));
  }

  function getEventLabel(eventName: WidgetInteractionEventName): string {
    switch (eventName) {
      case "click":
        return "CLK";
      case "doubleClick":
        return "DCLK";
      case "rightClick":
        return "RCLK";
      default:
        return eventName;
    }
  }

  function getActionLabel(action: WidgetEventAction): string {
    return ACTION_OPTIONS.find((option) => option.value === action.type)?.label ?? action.type;
  }

  function isBindingExpanded(index: number): boolean {
    return !collapsedBindings.includes(index);
  }

  function toggleBindingExpanded(index: number): void {
    if (isBindingExpanded(index)) {
      collapsedBindings = [...collapsedBindings, index];
      return;
    }

    collapsedBindings = collapsedBindings.filter((item) => item !== index);
  }

  function isBindingValid(binding: WidgetEventBinding): boolean {
    const action = getPrimaryAction(binding);
    if (action.type === "navigateRuntimeView") {
      return action.params.viewId.trim().length > 0;
    }

    if (action.type === "openContextMenu") {
      return (
        action.params.items.length > 0 &&
        action.params.items.every((item) => item.label.trim().length > 0)
      );
    }

    return true;
  }

  function clampPanelWidth(width: number): number {
    return Math.max(MIN_PANEL_WIDTH, Math.min(MAX_PANEL_WIDTH, width));
  }

  function startResize(event: PointerEvent): void {
    if (!visible) {
      return;
    }
    event.preventDefault();
    resizing = true;
    dragStartX = event.clientX;
    dragStartWidth = panelWidth;
  }

  function markInspectorInteractionStart(event: PointerEvent): void {
    if (event.button !== 0) {
      return;
    }
    suppressOutsideRelease = true;
  }

  onMount(() => {
    const shouldSuppressOutside = (event: Event): boolean => {
      if (!suppressOutsideRelease || !inspectorRoot) {
        return false;
      }
      const target = event.target;
      if (target instanceof Node && inspectorRoot.contains(target)) {
        return false;
      }
      event.preventDefault();
      event.stopPropagation();
      return true;
    };

    const onPointerMove = (event: PointerEvent): void => {
      if (!resizing) {
        return;
      }
      const deltaX = dragStartX - event.clientX;
      panelWidth = clampPanelWidth(dragStartWidth + deltaX);
    };

    const onPointerUp = (event: PointerEvent): void => {
      shouldSuppressOutside(event);
      resizing = false;
      suppressOutsideRelease = false;
    };

    const onClickCapture = (event: MouseEvent): void => {
      shouldSuppressOutside(event);
    };

    document.addEventListener("pointermove", onPointerMove);
    document.addEventListener("pointerup", onPointerUp, true);
    document.addEventListener("click", onClickCapture, true);

    return () => {
      document.removeEventListener("pointermove", onPointerMove);
      document.removeEventListener("pointerup", onPointerUp, true);
      document.removeEventListener("click", onClickCapture, true);
    };
  });
</script>

{#if visible}
  <aside
    bind:this={inspectorRoot}
    class="relative flex h-full shrink-0 flex-col border-l border-border bg-card p-3"
    style={`width:${panelWidth}px;`}
    onpointerdowncapture={markInspectorInteractionStart}
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
    <button
      type="button"
      class="absolute top-0 -left-1 h-full w-2 cursor-col-resize bg-transparent"
      title="Resize inspector"
      aria-label="Resize inspector"
      onpointerdown={(event) => startResize(event)}
    ></button>

    <div class="mb-3 flex items-center justify-between">
      <h3 class="text-sm font-semibold text-foreground">Graph Node Inspector</h3>
      <Button
        variant="icon"
        title="Hide inspector"
        ariaLabel="Hide inspector"
        label="×"
        onclick={onHide}
      />
    </div>

    {#if selectedEdgeStyle}
      <div class="min-h-0 flex-1 space-y-3 overflow-y-auto pr-1">
        <div class="rounded border border-border p-2">
          <p class="text-[10px] uppercase text-muted-foreground">CONNECTOR</p>
        </div>

        <div class="space-y-1.5">
          <p class="text-[10px] uppercase text-muted-foreground">Style</p>
          <Select.Root
            type="single"
            value={activeConnectorStyle()}
            onValueChange={(value) =>
              onConnectorStyleChange(selectedEdgeId, value as ConnectorStyle)}
          >
            <Select.Trigger class="h-8 w-full text-xs">
              {CONNECTOR_STYLE_OPTIONS.find((option) => option.value === activeConnectorStyle())?.label ??
                activeConnectorStyle()}
            </Select.Trigger>
            <Select.Content
              portalProps={{ disabled: true }}
              class="border border-border bg-card"
            >
              <Select.Group>
                {#each CONNECTOR_STYLE_OPTIONS as option (option.value)}
                  <Select.Item value={option.value} label={option.label} />
                {/each}
              </Select.Group>
            </Select.Content>
          </Select.Root>
        </div>

        {#if activeConnectorStyle() === "arrow"}
          {@const arrow = activeArrowConfig()}
          <div class="space-y-2 rounded border border-border bg-muted/10 p-2">
            <p class="text-[10px] uppercase text-muted-foreground">Arrow Configuration</p>

            <div class="space-y-1.5">
              <p class="text-xs text-foreground">Color</p>
              <div class="flex items-center gap-2">
                <input
                  type="color"
                  class="h-8 w-10 cursor-pointer rounded border border-input bg-transparent p-1"
                  value={arrow.color}
                  onchange={(event) =>
                    onArrowConnectorConfigChange(selectedEdgeId, {
                      color: (event.currentTarget as HTMLInputElement).value,
                    })}
                />
                <Input
                  class="h-8 text-xs"
                  value={arrow.color}
                  oninput={(event) =>
                    onArrowConnectorConfigChange(selectedEdgeId, {
                      color: (event.currentTarget as HTMLInputElement).value,
                    })}
                />
              </div>
            </div>

            <div class="grid grid-cols-2 gap-2">
              <div class="space-y-1">
                <p class="block text-[10px] uppercase text-muted-foreground">Thickness</p>
                <NumberField
                  class="w-full text-xs"
                  min={1}
                  max={20}
                  step={1}
                  value={arrow.thickness}
                  onValueChange={(value) => {
                    if (typeof value === "number") {
                      onArrowConnectorConfigChange(selectedEdgeId, { thickness: value });
                    }
                  }}
                />
              </div>
              <div class="space-y-1">
                <p class="block text-[10px] uppercase text-muted-foreground">Arrow size</p>
                <NumberField
                  class="w-full text-xs"
                  min={6}
                  max={64}
                  step={1}
                  value={arrow.arrowSize}
                  onValueChange={(value) => {
                    if (typeof value === "number") {
                      onArrowConnectorConfigChange(selectedEdgeId, { arrowSize: value });
                    }
                  }}
                />
              </div>
            </div>
          </div>
        {:else}
          {@const pipe = activePipeConfig()}
          <div class="space-y-2 rounded border border-border bg-muted/10 p-2">
            <p class="text-[10px] uppercase text-muted-foreground">Pipe Configuration</p>
            <div class="grid grid-cols-2 gap-2">
              <div class="space-y-1">
                <p class="block text-[10px] uppercase text-muted-foreground">Thickness</p>
                <NumberField
                  class="w-full text-xs"
                  min={2}
                  max={64}
                  step={1}
                  value={pipe.thickness}
                  onValueChange={(value) => {
                    if (typeof value === "number") {
                      onPipeConnectorConfigChange(selectedEdgeId, { thickness: value });
                    }
                  }}
                />
              </div>
              <div class="space-y-1">
                <p class="block text-[10px] uppercase text-muted-foreground">Flange size</p>
                <NumberField
                  class="w-full text-xs"
                  min={1}
                  max={8}
                  step={1}
                  value={pipe.flangeScale}
                  onValueChange={(value) => {
                    if (typeof value === "number") {
                      onPipeConnectorConfigChange(selectedEdgeId, { flangeScale: value });
                    }
                  }}
                />
              </div>
            </div>
          </div>
        {/if}
      </div>
    {:else if selectedNodeData && selectedWidgetDefinition}
      <div class="min-h-0 flex-1 space-y-3 overflow-y-auto pr-1">
        <div class="rounded border border-border p-2">
          <p class="text-[10px] uppercase text-muted-foreground">Widget</p>
          <p class="text-xs font-medium text-foreground">
            {selectedWidgetDefinition.label}
          </p>
        </div>

        <div>
          <label
            for="node-label-input"
            class="mb-1 block text-[10px] uppercase text-muted-foreground"
            >Label</label
          >
          <Input
            id="node-label-input"
            class="w-full text-xs"
            value={selectedNodeData.title}
            oninput={(event) =>
              onLabelChange((event.currentTarget as HTMLInputElement).value)}
          />
        </div>

        <div>
          <p class="mb-1 block text-[10px] uppercase text-muted-foreground">Size</p>
          <div class="grid grid-cols-2 gap-2">
            <div class="space-y-1">
              <label for="node-width-input" class="block text-[10px] uppercase text-muted-foreground"
                >W</label
              >
              <NumberField
                id="node-width-input"
                class="w-full text-xs"
                min={getMinNodeWidth()}
                step={1}
                value={effectiveNodeWidth()}
                onValueChange={(value) => {
                  if (typeof value === "number") {
                    onDimensionsChange(selectedNodeId, { width: value });
                  }
                }}
              />
            </div>
            <div class="space-y-1">
              <label for="node-height-input" class="block text-[10px] uppercase text-muted-foreground"
                >H</label
              >
              <NumberField
                id="node-height-input"
                class="w-full text-xs"
                min={getMinNodeHeight()}
                step={1}
                value={effectiveNodeHeight()}
                onValueChange={(value) => {
                  if (typeof value === "number") {
                    onDimensionsChange(selectedNodeId, { height: value });
                  }
                }}
              />
            </div>
          </div>
        </div>

        <div>
          <p class="mb-1 block text-[10px] uppercase text-muted-foreground">Ports (%)</p>
          <div class="grid grid-cols-2 gap-2">
            <div class="space-y-1">
              <label for="node-port-top-input" class="block text-[10px] uppercase text-muted-foreground"
                >Top</label
              >
              <NumberField
                id="node-port-top-input"
                class="w-full text-xs"
                min={0}
                max={100}
                step={1}
                value={portOffset("top")}
                onValueChange={(value) => {
                  if (typeof value === "number") {
                    onPortOffsetsChange(selectedNodeId, { top: value });
                  }
                }}
              />
            </div>
            <div class="space-y-1">
              <label for="node-port-right-input" class="block text-[10px] uppercase text-muted-foreground"
                >Right</label
              >
              <NumberField
                id="node-port-right-input"
                class="w-full text-xs"
                min={0}
                max={100}
                step={1}
                value={portOffset("right")}
                onValueChange={(value) => {
                  if (typeof value === "number") {
                    onPortOffsetsChange(selectedNodeId, { right: value });
                  }
                }}
              />
            </div>
            <div class="space-y-1">
              <label for="node-port-bottom-input" class="block text-[10px] uppercase text-muted-foreground"
                >Bottom</label
              >
              <NumberField
                id="node-port-bottom-input"
                class="w-full text-xs"
                min={0}
                max={100}
                step={1}
                value={portOffset("bottom")}
                onValueChange={(value) => {
                  if (typeof value === "number") {
                    onPortOffsetsChange(selectedNodeId, { bottom: value });
                  }
                }}
              />
            </div>
            <div class="space-y-1">
              <label for="node-port-left-input" class="block text-[10px] uppercase text-muted-foreground"
                >Left</label
              >
              <NumberField
                id="node-port-left-input"
                class="w-full text-xs"
                min={0}
                max={100}
                step={1}
                value={portOffset("left")}
                onValueChange={(value) => {
                  if (typeof value === "number") {
                    onPortOffsetsChange(selectedNodeId, { left: value });
                  }
                }}
              />
            </div>
          </div>
        </div>

        <div class="space-y-2">
          {#each selectedWidgetDefinition.bindings as binding (binding.key)}
            {@const bindingTags = selectedNodeData.bindings?.[binding.key] ?? []}
            <div class="rounded border border-border p-2">
              <div class="mb-1 flex items-center justify-between">
                <span class="font-medium text-foreground">{binding.label}</span>
                <span class="text-[10px] uppercase text-muted-foreground">
                  {binding.access}
                </span>
              </div>

              {#if binding.multiple}
                <div
                  class="min-h-[38px] rounded border border-dashed border-border/60 bg-muted/50 px-2 py-1"
                  role="group"
                  ondragover={(event) => {
                    event.preventDefault();
                    event.stopPropagation();
                    if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
                  }}
                  ondrop={(event) =>
                    onBindingDrop(event, selectedNodeId, binding.key)}
                >
                  {#if bindingTags.length === 0}
                    <span class="text-[10px] text-muted-foreground">
                      Drop Variable(s) here
                    </span>
                  {:else}
                    <div class="flex flex-wrap gap-1">
                      {#each bindingTags as tag (tag.id)}
                        <span
                          class="inline-flex items-center gap-1 rounded bg-primary/15 px-1.5 py-0.5 text-[10px] text-foreground"
                        >
                          <span class="max-w-[160px] truncate">{tag.name}</span>
                          <button
                            type="button"
                            class="text-[10px] text-destructive/85 opacity-90 hover:text-destructive"
                            title="Remove binding"
                            onclick={() =>
                              onRemoveTagFromBinding(
                                selectedNodeId,
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
                <div class="flex items-center gap-1">
                  <Input
                    class="w-full text-[10px]"
                    readonly
                    value={bindingTags[0]?.path ?? ""}
                    placeholder="Drop Variable here"
                    ondragover={(event) => {
                      event.preventDefault();
                      event.stopPropagation();
                      if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
                    }}
                    ondrop={(event) =>
                      onBindingDrop(event, selectedNodeId, binding.key)}
                  />
                  {#if bindingTags[0]}
                    <Button
                      variant="icon"
                      class="shrink-0"
                      label="×"
                      title="Clear binding"
                      ariaLabel="Clear binding"
                      onclick={() =>
                        onRemoveTagFromBinding(
                          selectedNodeId,
                          binding.key,
                          bindingTags[0].id,
                        )}
                    />
                  {/if}
                </div>
              {/if}

              {#if binding.required}
                <p class="mt-1 text-[10px] text-muted-foreground">Required</p>
              {/if}
            </div>
          {/each}
        </div>

        {#if activeConfigSchema().length > 0}
          <div class="space-y-2">
            <div class="flex items-center justify-between">
              <p class="text-[10px] uppercase text-muted-foreground">Widget Configuration</p>
            </div>

            {#each activeConfigSchema() as field (field.key)}
              <div class="space-y-1.5">
                {#if field.type !== "boolean"}
                  <div class="flex items-center justify-between gap-2">
                    <p class="text-xs font-medium text-foreground">{field.label}</p>
                  </div>
                {/if}
                {#if field.description}
                  <p class="text-[10px] text-muted-foreground">{field.description}</p>
                {/if}

                {#if field.type === "boolean"}
                  {@const checked = Boolean(getConfigValue(field))}
                  <label class="flex items-center gap-2 text-xs text-foreground">
                    <input
                      type="checkbox"
                      checked={checked}
                      onchange={(event) =>
                        updateConfigField(
                          field.key,
                          (event.currentTarget as HTMLInputElement).checked,
                        )}
                    />
                    <span>{field.label}</span>
                  </label>
                {:else if field.type === "number"}
                  <Input
                    class="h-8 text-xs"
                    type="number"
                    min={field.min}
                    max={field.max}
                    step={field.step ?? "any"}
                    value={String(getConfigValue(field) ?? "")}
                    placeholder={field.placeholder ?? ""}
                    oninput={(event) => {
                      const raw = (event.currentTarget as HTMLInputElement).value;
                      if (raw.trim().length === 0) {
                        updateConfigField(field.key, undefined);
                        return;
                      }
                      const parsed = Number(raw);
                      if (Number.isFinite(parsed)) {
                        updateConfigField(field.key, parsed);
                      }
                    }}
                  />
                {:else if field.type === "color"}
                  {@const currentColor = String(getConfigValue(field) ?? "#546a0e")}
                  <div class="flex items-center gap-2">
                    <input
                      type="color"
                      class="h-8 w-10 cursor-pointer rounded border border-input bg-transparent p-1"
                      value={currentColor}
                      onchange={(event) =>
                        updateConfigField(
                          field.key,
                          (event.currentTarget as HTMLInputElement).value,
                        )}
                    />
                    <Input
                      class="h-8 text-xs"
                      value={currentColor}
                      oninput={(event) =>
                        updateConfigField(
                          field.key,
                          (event.currentTarget as HTMLInputElement).value,
                        )}
                    />
                  </div>
                {:else if field.type === "select"}
                  {@const selectedValue = String(getConfigValue(field) ?? "")}
                  <Select.Root
                    type="single"
                    value={selectedValue}
                    onValueChange={(value) => updateConfigField(field.key, value)}
                  >
                    <Select.Trigger class="h-8 w-full text-xs">
                      {getSelectOptionLabel(field, selectedValue)}
                    </Select.Trigger>
                    <Select.Content
                      portalProps={{ disabled: true }}
                      class="border border-border bg-card"
                    >
                      <Select.Group>
                        {#each field.options ?? [] as option (option.value)}
                          <Select.Item value={option.value} label={option.label} />
                        {/each}
                      </Select.Group>
                    </Select.Content>
                  </Select.Root>
                {:else if field.type === "thresholds"}
                  {@const ranges = readThresholdRanges(field)}
                  <div class="space-y-2">
                    <div class="flex items-center justify-between">
                      <p class="text-[10px] uppercase text-muted-foreground">Intervals</p>
                      <Button
                        variant="outline-accent"
                        label="Add"
                        class="h-7 px-2 text-[10px]"
                        onclick={() => addThresholdRange(field)}
                      />
                    </div>

                    {#if ranges.length === 0}
                      <p class="text-[10px] text-muted-foreground">
                        Add at least one interval.
                      </p>
                    {/if}

                    {#each ranges as range, rangeIndex (`${field.key}-${rangeIndex}`)}
                      <div class="space-y-2 rounded border border-border/50 bg-card/70 p-2">
                        <div class="grid grid-cols-[1fr_1fr_1fr_40px] items-center gap-2">
                          <Input
                            class="h-8 text-xs"
                            type="number"
                            step="any"
                            placeholder="From"
                            value={range.min ?? ""}
                            oninput={(event) => {
                              const raw = (event.currentTarget as HTMLInputElement).value;
                              updateThresholdRange(field, rangeIndex, {
                                min: raw.trim().length ? Number(raw) : undefined,
                              });
                            }}
                          />
                          <Input
                            class="h-8 text-xs"
                            type="number"
                            step="any"
                            placeholder="To"
                            value={range.max ?? ""}
                            oninput={(event) => {
                              const raw = (event.currentTarget as HTMLInputElement).value;
                              updateThresholdRange(field, rangeIndex, {
                                max: raw.trim().length ? Number(raw) : undefined,
                              });
                            }}
                          />
                          <div class="flex items-center gap-1">
                            <input
                              type="color"
                              class="h-8 w-9 cursor-pointer rounded border border-input bg-transparent p-1"
                              value={range.color}
                              onchange={(event) =>
                                updateThresholdRange(field, rangeIndex, {
                                  color: (event.currentTarget as HTMLInputElement).value,
                                })}
                            />
                            <Input
                              class="h-8 text-xs"
                              value={range.color}
                              oninput={(event) =>
                                updateThresholdRange(field, rangeIndex, {
                                  color: (event.currentTarget as HTMLInputElement).value,
                                })}
                            />
                          </div>
                          <Button
                            variant="icon"
                            icon={Trash2}
                            class="border-destructive/45 text-destructive hover:border-destructive/70 hover:bg-destructive/12 hover:text-destructive"
                            title="Remove interval"
                            ariaLabel="Remove interval"
                            onclick={() => removeThresholdRange(field, rangeIndex)}
                          />
                        </div>
                        <Input
                          class="h-8 text-xs"
                          placeholder="Optional label"
                          value={range.label ?? ""}
                          oninput={(event) =>
                            updateThresholdRange(field, rangeIndex, {
                              label: (event.currentTarget as HTMLInputElement).value,
                            })}
                        />
                      </div>
                    {/each}
                  </div>
                {:else}
                  {#if field.type === "textarea"}
                    <Textarea
                      class="min-h-24 text-xs"
                      value={String(getConfigValue(field) ?? "")}
                      placeholder={field.placeholder ?? ""}
                      oninput={(event) =>
                        updateConfigField(
                          field.key,
                          (event.currentTarget as HTMLTextAreaElement).value,
                        )}
                    />
                  {:else}
                    <Input
                      class="h-8 text-xs"
                      value={String(getConfigValue(field) ?? "")}
                      placeholder={field.placeholder ?? ""}
                      oninput={(event) =>
                        updateConfigField(
                          field.key,
                          (event.currentTarget as HTMLInputElement).value,
                        )}
                    />
                  {/if}
                {/if}
              </div>
            {/each}
          </div>
        {/if}

        {#if availableEventOptions().length > 0}
          <div class="space-y-2 rounded border border-border bg-muted/10 p-2">
          <div class="flex items-center justify-between">
            <p class="text-[10px] uppercase text-muted-foreground">Event Bindings</p>
            <Button
              variant="outline-accent"
              label="Add"
              class="h-7 px-2 text-[10px]"
              disabled={currentBindings().length >= availableEventOptions().length}
              onclick={addInteractionBinding}
            />
          </div>

          {#if currentBindings().length === 0}
            <p class="text-[10px] text-muted-foreground">
              No interaction bindings configured.
            </p>
          {/if}

          {#each currentBindings() as binding, bindingIndex (`binding-${bindingIndex}`)}
            {@const action = getPrimaryAction(binding)}
            <section class="rounded border border-border/60 bg-card/60 p-2">
              <div class="flex items-center justify-between gap-2">
                <button
                  type="button"
                  class="flex min-w-0 flex-1 items-center gap-2 rounded px-1 py-1 text-left hover:bg-muted/40"
                  onclick={() => toggleBindingExpanded(bindingIndex)}
                  title={isBindingExpanded(bindingIndex) ? "Collapse details" : "Expand details"}
                >
                  <span class="rounded border border-border bg-muted/40 px-2 py-0.5 text-[10px] uppercase text-foreground">
                    {getEventLabel(binding.on)}
                  </span>
                  <span class="text-[10px] text-muted-foreground">→</span>
                  <span class="truncate rounded border border-border bg-muted/30 px-2 py-0.5 text-[10px] text-foreground">
                    {getActionLabel(action)}
                  </span>
                  <span
                    class={`ml-auto rounded px-2 py-0.5 text-[10px] uppercase ${isBindingValid(binding) ? "bg-primary/15 text-primary" : "bg-destructive/15 text-destructive"}`}
                  >
                    {isBindingValid(binding) ? "Valid" : "Needs config"}
                  </span>
                </button>
                <Button
                  variant="icon"
                  icon={Trash2}
                  class="border-destructive/45 text-destructive hover:border-destructive/70 hover:bg-destructive/12 hover:text-destructive"
                  title="Remove binding"
                  ariaLabel="Remove binding"
                  onclick={() => removeInteractionBinding(bindingIndex)}
                />
              </div>

              {#if isBindingExpanded(bindingIndex)}
                <div class="mt-2 space-y-2 rounded bg-muted/10 p-2" transition:slide={{ duration: 140 }}>
                  <div class="grid grid-cols-2 gap-2">
                    <div>
                      <label for={`event-binding-${bindingIndex}`} class="mb-1 block text-[10px] uppercase text-muted-foreground">Event</label>
                      <select
                        id={`event-binding-${bindingIndex}`}
                        class="h-8 w-full rounded border border-input bg-transparent px-2 text-xs"
                        value={binding.on}
                        onchange={(event) =>
                          updateEventType(bindingIndex, (event.currentTarget as HTMLSelectElement).value)}
                      >
                        {#each availableEventOptions() as option (option.value)}
                          {@const takenElsewhere = option.value !== binding.on && isEventAlreadyBound(option.value, bindingIndex)}
                          <option
                            value={option.value}
                            disabled={takenElsewhere}
                          >
                            {option.label}{takenElsewhere ? " (used)" : ""}
                          </option>
                        {/each}
                      </select>
                    </div>

                    <div>
                      <label for={`event-action-${bindingIndex}`} class="mb-1 block text-[10px] uppercase text-muted-foreground">Action</label>
                      <select
                        id={`event-action-${bindingIndex}`}
                        class="h-8 w-full rounded border border-input bg-transparent px-2 text-xs"
                        value={action.type}
                        onchange={(event) =>
                          updateActionType(bindingIndex, (event.currentTarget as HTMLSelectElement).value)}
                      >
                        {#each ACTION_OPTIONS as option (option.value)}
                          <option value={option.value}>{option.label}</option>
                        {/each}
                      </select>
                    </div>
                  </div>

                  {#if action.type === "navigateRuntimeView"}
                    <div>
                      <p class="mb-1 text-[10px] uppercase text-muted-foreground">Target Runtime View</p>
                      <Autocomplete
                        value={action.params.viewId}
                        selectedLabel={action.params.viewName ?? action.params.viewId}
                        placeholder="Type a view name"
                        emptyText="No views found"
                        searchItems={searchViews}
                        onValueChange={(item) => updateNavigateTarget(bindingIndex, item)}
                      />
                    </div>
                  {:else if action.type === "openContextMenu"}
                    <div class="space-y-2 rounded border border-border/50 bg-muted/15 p-2">
                      <div class="flex items-center justify-between">
                        <p class="text-[10px] uppercase text-muted-foreground">Menu Options</p>
                        <Button
                          variant="outline-accent"
                          label="Add option"
                          class="h-7 px-2 text-[10px]"
                          onclick={() => addContextMenuItem(bindingIndex)}
                        />
                      </div>

                      {#if action.params.items.length === 0}
                        <p class="text-[10px] text-muted-foreground">No options configured.</p>
                      {/if}

                      <div class="space-y-1.5">
                        {#each action.params.items as option, optionIndex (option.id)}
                          <div class="grid grid-cols-[18px_1fr_1fr_36px] items-center gap-2 rounded border border-border/50 bg-card/70 p-1.5">
                            <span class="text-center text-[11px] text-muted-foreground">⋮⋮</span>
                            <Input
                              class="h-8 text-xs"
                              value={option.label}
                              placeholder="Option label"
                              oninput={(event) =>
                                updateContextMenuItemLabel(
                                  bindingIndex,
                                  optionIndex,
                                  (event.currentTarget as HTMLInputElement).value,
                                )}
                            />
                            <Autocomplete
                              value={option.viewId}
                              selectedLabel={option.viewName ?? option.viewId}
                              placeholder="Optional runtime view target"
                              emptyText="No views found"
                              searchItems={searchViews}
                              onValueChange={(item) =>
                                updateContextMenuItemView(bindingIndex, optionIndex, item)}
                            />
                            <Button
                              variant="icon"
                              icon={Trash2}
                              class="border-destructive/45 text-destructive hover:border-destructive/70 hover:bg-destructive/12 hover:text-destructive"
                              title="Remove option"
                              ariaLabel="Remove option"
                              onclick={() => removeContextMenuItem(bindingIndex, optionIndex)}
                            />
                          </div>
                        {/each}
                      </div>
                    </div>
                  {/if}
                </div>
              {/if}
            </section>
          {/each}
          </div>
        {/if}
      </div>
    {:else}
      <div
        class="flex min-h-0 flex-1 items-center justify-center text-center text-sm text-muted-foreground"
      >
        Select a node or connector in the graph to configure it.
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
        onclick={onShow}
      />
    </div>
  </div>
{/if}
