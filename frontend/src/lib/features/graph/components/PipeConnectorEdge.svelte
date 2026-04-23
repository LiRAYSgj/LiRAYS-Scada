<script lang="ts">
  import { getSmoothStepPath } from "@xyflow/system";
  import type { EdgeProps, Position } from "@xyflow/svelte";
  import {
    DEFAULT_PIPE_CONNECTOR,
    resolveConnectorData,
  } from "$lib/features/graph/connectors";

  let {
    id,
    data,
    selected = false,
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  }: EdgeProps = $props();

  interface FlangeRect {
    x: number;
    y: number;
    width: number;
    height: number;
    rx: number;
  }

  let connector = $derived(
    resolveConnectorData({
      id,
      type: "connector-pipe",
      style: undefined,
      data,
    }),
  );
  let pipeThickness = $derived(
    Math.max(2, connector.pipe?.thickness ?? DEFAULT_PIPE_CONNECTOR.thickness),
  );
  let flangeScale = $derived(
    Math.round(
      Math.max(
        1,
        connector.pipe?.flangeScale ?? DEFAULT_PIPE_CONNECTOR.flangeScale,
      ),
    ),
  );

  function moveFromHandle(
    x: number,
    y: number,
    pos: Position,
    distance: number,
  ): {
    x: number;
    y: number;
  } {
    if (pos === "left") return { x: x - distance, y };
    if (pos === "right") return { x: x + distance, y };
    if (pos === "top") return { x, y: y - distance };
    return { x, y: y + distance };
  }

  function flangeRectAt(
    x: number,
    y: number,
    position: Position,
    thickness: number,
    scale: number,
  ): FlangeRect {
    const across = Math.max(14, Math.min(72, thickness * 1.05 * scale));
    const along = Math.max(8, Math.min(30, thickness * 0.36));
    const horizontalAxis = position === "left" || position === "right";
    const width = horizontalAxis ? along : across;
    const height = horizontalAxis ? across : along;
    return {
      x: x - width / 2,
      y: y - height / 2,
      width,
      height,
      rx: Math.max(1.8, Math.min(4, Math.min(width, height) * 0.24)),
    };
  }

  function flangeDepth(rect: FlangeRect, position: Position): number {
    return position === "left" || position === "right"
      ? rect.width
      : rect.height;
  }

  let sourceFlange = $derived(
    flangeRectAt(sourceX, sourceY, sourcePosition, pipeThickness, flangeScale),
  );
  let targetFlange = $derived(
    flangeRectAt(targetX, targetY, targetPosition, pipeThickness, flangeScale),
  );

  let sourceCapOffset = $derived(
    Math.max(
      10,
      flangeDepth(sourceFlange, sourcePosition) * 0.5 + pipeThickness * 0.5 + 2,
    ),
  );
  let targetCapOffset = $derived(
    Math.max(
      10,
      flangeDepth(targetFlange, targetPosition) * 0.5 + pipeThickness * 0.5 + 2,
    ),
  );
  let routedSource = $derived(
    moveFromHandle(sourceX, sourceY, sourcePosition, sourceCapOffset),
  );
  let routedTarget = $derived(
    moveFromHandle(targetX, targetY, targetPosition, targetCapOffset),
  );

  let [path] = $derived(
    getSmoothStepPath({
      sourceX: routedSource.x,
      sourceY: routedSource.y,
      targetX: routedTarget.x,
      targetY: routedTarget.y,
      sourcePosition,
      targetPosition,
      borderRadius: Math.max(8, Math.min(28, pipeThickness * 0.8)),
    }),
  );
  let sourceStubPath = $derived(
    `M ${sourceX} ${sourceY} L ${routedSource.x} ${routedSource.y}`,
  );
  let targetStubPath = $derived(
    `M ${routedTarget.x} ${routedTarget.y} L ${targetX} ${targetY}`,
  );
  let fullPipePath = $derived(`${sourceStubPath} ${path} ${targetStubPath}`);

  let safeId = $derived(
    `connector-pipe-${(id ?? "edge").replace(/[^a-zA-Z0-9_-]/g, "-")}`,
  );
  let flangeGradientId = $derived(`${safeId}-flange-linear`);
  let interactionWidth = $derived(Math.max(22, pipeThickness + 14));
  const SELECT_BLUE = "#3b82f6";
  const SELECT_BORDER = 2.4;
  const SELECT_PADDING = 1.2;
  let pipeBandWidths = $derived.by(() => {
    const t = pipeThickness;
    const outer = t;

    const dark = t * 0.8;
    const mid = t * 0.58;
    const light = t * 0.32;
    const core = t * 0.14;
    return { outer, dark, mid, light, core };
  });
  let pipeBandOuter = $derived(pipeBandWidths.outer);
  let pipeBandDark = $derived(pipeBandWidths.dark);
  let pipeBandMid = $derived(pipeBandWidths.mid);
  let pipeBandLight = $derived(pipeBandWidths.light);
  let pipeBandCore = $derived(pipeBandWidths.core);
</script>

<defs>
  <linearGradient id={flangeGradientId} x1="0%" y1="0%" x2="100%" y2="0%">
    <stop offset="0" stop-color="#2d2f2a" />
    <stop offset="0.5" stop-color="#b6b6a2" />
    <stop offset="1" stop-color="#202417" />
  </linearGradient>
</defs>

{#if selected}
  <rect
    x={sourceFlange.x - SELECT_PADDING}
    y={sourceFlange.y - SELECT_PADDING}
    width={sourceFlange.width + SELECT_PADDING * 2}
    height={sourceFlange.height + SELECT_PADDING * 2}
    rx={sourceFlange.rx + 0.8}
    fill="none"
    stroke={SELECT_BLUE}
    stroke-width={SELECT_BORDER}
    opacity="0.98"
  />
  <rect
    x={targetFlange.x - SELECT_PADDING}
    y={targetFlange.y - SELECT_PADDING}
    width={targetFlange.width + SELECT_PADDING * 2}
    height={targetFlange.height + SELECT_PADDING * 2}
    rx={targetFlange.rx + 0.8}
    fill="none"
    stroke={SELECT_BLUE}
    stroke-width={SELECT_BORDER}
    opacity="0.98"
  />
{/if}

<path
  {id}
  d={fullPipePath}
  class="connector-pipe-path"
  fill="none"
  stroke="#2d311f"
  stroke-width={pipeBandOuter}
  stroke-linecap="butt"
  stroke-linejoin="round"
/>
<path
  d={fullPipePath}
  fill="none"
  stroke="#3f4827"
  stroke-width={pipeBandDark}
  stroke-linecap="butt"
  stroke-linejoin="round"
/>
<path
  d={fullPipePath}
  fill="none"
  stroke="#727a5e"
  stroke-width={pipeBandMid}
  stroke-linecap="butt"
  stroke-linejoin="round"
/>
<path
  d={fullPipePath}
  fill="none"
  stroke="#aab095"
  stroke-width={pipeBandLight}
  stroke-linecap="butt"
  stroke-linejoin="round"
/>
<path
  d={fullPipePath}
  fill="none"
  stroke="#d2d6c0"
  stroke-width={pipeBandCore}
  stroke-linecap="butt"
  stroke-linejoin="round"
/>

<rect
  x={sourceFlange.x}
  y={sourceFlange.y}
  width={sourceFlange.width}
  height={sourceFlange.height}
  rx={sourceFlange.rx}
  fill={`url(#${flangeGradientId})`}
  stroke="#20231a"
  stroke-width="1.2"
/>

<rect
  x={targetFlange.x}
  y={targetFlange.y}
  width={targetFlange.width}
  height={targetFlange.height}
  rx={targetFlange.rx}
  fill={`url(#${flangeGradientId})`}
  stroke="#20231a"
  stroke-width="1.2"
/>

{#if selected}
  <path
    d={fullPipePath}
    fill="none"
    stroke={SELECT_BLUE}
    stroke-width={Math.max(SELECT_BORDER, pipeBandOuter + SELECT_BORDER * 1.3)}
    stroke-linecap="round"
    stroke-linejoin="round"
    opacity="0.9"
  />
{/if}

<path
  d={fullPipePath}
  class="svelte-flow__edge-interaction"
  stroke-opacity={0}
  stroke-width={interactionWidth}
  fill="none"
/>
