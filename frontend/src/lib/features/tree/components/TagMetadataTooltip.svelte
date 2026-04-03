<script lang="ts">
  import type { TreeNode } from "../types";
  import OverlaySurface from "./OverlaySurface.svelte";

  interface Props {
    x: number;
    y: number;
    node: TreeNode;
    width?: number;
  }

  let { x, y, node, width = 240 }: Props = $props();
</script>

<OverlaySurface
  {x}
  {y}
  {width}
  zIndex={45}
  pointerEvents="none"
  className="rounded-md border border-border bg-card px-3 py-2 text-[11px] shadow-lg"
>
  <div class="space-y-1 text-muted-foreground">
    <div class="flex justify-between gap-2">
      <span class="text-muted-foreground">Unit</span>
      <span>{node.unit ?? "-"}</span>
    </div>
    {#if node.dataType === "VAR_DATA_TYPE_INTEGER" || node.dataType === "VAR_DATA_TYPE_FLOAT"}
      <div class="flex justify-between gap-2">
        <span class="text-muted-foreground">Min</span>
        <span>{node.min ?? "-"}</span>
      </div>
      <div class="flex justify-between gap-2">
        <span class="text-muted-foreground">Max</span>
        <span>{node.max ?? "-"}</span>
      </div>
    {/if}
    {#if node.dataType === "VAR_DATA_TYPE_TEXT"}
      <div class="flex justify-between gap-2">
        <span class="text-muted-foreground">Max len</span>
        <span>{node.maxLen && node.maxLen.length > 0 ? node.maxLen[0] : "-"}</span>
      </div>
      <div>
        <div class="text-muted-foreground">Options</div>
        <div class="truncate text-muted-foreground">
          {node.options && node.options.length ? node.options.join(", ") : "-"}
        </div>
      </div>
    {/if}
  </div>
</OverlaySurface>
