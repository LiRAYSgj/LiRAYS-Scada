<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    x: number;
    y: number;
    width?: number | string;
    zIndex?: number;
    className?: string;
    style?: string;
    pointerEvents?: "auto" | "none";
    dataContextMenu?: boolean;
    children?: Snippet;
  }

  let {
    x,
    y,
    width,
    zIndex = 50,
    className = "",
    style = "",
    pointerEvents = "auto",
    dataContextMenu = false,
    children,
  }: Props = $props();

  const widthStyle = $derived.by(() => {
    if (width === undefined) return "";
    if (typeof width === "number") return `width:${width}px;`;
    return `width:${width};`;
  });

  const mergedStyle = $derived(
    `left:${x}px;top:${y}px;z-index:${zIndex};pointer-events:${pointerEvents};${widthStyle}${style}`,
  );
</script>

<div
  data-context-menu={dataContextMenu ? "" : undefined}
  class={`fixed ${className}`.trim()}
  style={mergedStyle}
>
  {@render children?.()}
</div>
