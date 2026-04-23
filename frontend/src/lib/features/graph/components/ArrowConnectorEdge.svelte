<script lang="ts">
  import { getSmoothStepPath } from "@xyflow/system";
  import type { EdgeProps, Position } from "@xyflow/svelte";
  import { resolveConnectorData } from "$lib/features/graph/connectors";

  interface Point {
    x: number;
    y: number;
  }

  let {
    id,
    data,
    style,
    selected = false,
    sourceX,
    sourceY,
    sourcePosition,
    targetX,
    targetY,
    targetPosition,
  }: EdgeProps = $props();

  function moveFromHandle(x: number, y: number, pos: Position, distance: number): Point {
    if (pos === "left") return { x: x - distance, y };
    if (pos === "right") return { x: x + distance, y };
    if (pos === "top") return { x, y: y - distance };
    return { x, y: y + distance };
  }

  let connector = $derived(
    resolveConnectorData({
      id,
      type: "connector-arrow",
      style,
      data,
    }),
  );

  let arrowDepth = $derived(Math.max(8, connector.arrow.arrowSize * 0.95));
  let arrowWidth = $derived(Math.max(10, connector.arrow.arrowSize * 1.05));
  let strokeWidth = $derived(Math.max(1, connector.arrow.thickness));
  let tipClearance = $derived(Math.max(0.8, strokeWidth * 0.3));

  let routedSource = $derived({ x: sourceX, y: sourceY });
  let routedTarget = $derived(
    moveFromHandle(
      targetX,
      targetY,
      targetPosition,
      arrowDepth + tipClearance,
    ),
  );
  let arrowBaseCenter = $derived(
    moveFromHandle(targetX, targetY, targetPosition, arrowDepth),
  );
  let [path] = $derived(
    getSmoothStepPath({
      sourceX: routedSource.x,
      sourceY: routedSource.y,
      targetX: routedTarget.x,
      targetY: routedTarget.y,
      sourcePosition,
      targetPosition,
      borderRadius: 0,
    }),
  );

  let arrowPolygonPoints = $derived.by(() => {
    const half = arrowWidth / 2;
    const tip = { x: targetX, y: targetY };
    const baseCenter = arrowBaseCenter;
    let a: Point;
    let b: Point;

    if (targetPosition === "top" || targetPosition === "bottom") {
      a = { x: baseCenter.x - half, y: baseCenter.y };
      b = { x: baseCenter.x + half, y: baseCenter.y };
    } else {
      a = { x: baseCenter.x, y: baseCenter.y - half };
      b = { x: baseCenter.x, y: baseCenter.y + half };
    }

    return `${Math.round(tip.x)},${Math.round(tip.y)} ${Math.round(a.x)},${Math.round(a.y)} ${Math.round(b.x)},${Math.round(b.y)}`;
  });

  let interactionWidth = $derived(Math.max(22, strokeWidth + 14));
  const SELECT_BLUE = "#3b82f6";
  const SELECT_BORDER = 2.4;
  let selectedOutlineWidth = $derived(strokeWidth + SELECT_BORDER * 2);
  let selectedArrowOutlineWidth = $derived(SELECT_BORDER * 2);
</script>

{#if selected}
  <path
    d={path}
    fill="none"
    stroke={SELECT_BLUE}
    stroke-width={selectedOutlineWidth}
    stroke-linecap="round"
    stroke-linejoin="round"
    opacity="0.98"
  />
  <polygon
    points={arrowPolygonPoints}
    fill="none"
    stroke={SELECT_BLUE}
    stroke-width={selectedArrowOutlineWidth}
    stroke-linejoin="round"
    opacity="0.98"
  />
{/if}

<path
  {id}
  d={path}
  class="svelte-flow__edge-path"
  fill="none"
  {style}
/>

<polygon
  points={arrowPolygonPoints}
  fill={connector.arrow.color}
  stroke={connector.arrow.color}
  stroke-width={Math.max(1, strokeWidth * 0.5)}
  stroke-linejoin="round"
/>

<path
  d={path}
  class="svelte-flow__edge-interaction"
  stroke-opacity={0}
  stroke-width={interactionWidth}
  fill="none"
/>
