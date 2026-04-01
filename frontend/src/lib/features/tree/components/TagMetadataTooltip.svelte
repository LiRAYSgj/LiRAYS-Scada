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
  className="rounded-md border border-black/10 bg-(--bg-panel) px-3 py-2 text-[11px] shadow-lg dark:border-white/10"
  style="background-color: var(--bg-panel);"
>
  <div class="space-y-1 text-(--text-secondary)">
    <div class="flex justify-between gap-2">
      <span class="text-(--text-muted)">Unit</span>
      <span>{node.unit ?? "-"}</span>
    </div>
    {#if node.dataType === "VAR_DATA_TYPE_INTEGER" || node.dataType === "VAR_DATA_TYPE_FLOAT"}
      <div class="flex justify-between gap-2">
        <span class="text-(--text-muted)">Min</span>
        <span>{node.min ?? "-"}</span>
      </div>
      <div class="flex justify-between gap-2">
        <span class="text-(--text-muted)">Max</span>
        <span>{node.max ?? "-"}</span>
      </div>
    {/if}
    {#if node.dataType === "VAR_DATA_TYPE_TEXT"}
      <div class="flex justify-between gap-2">
        <span class="text-(--text-muted)">Max len</span>
        <span>{node.maxLen && node.maxLen.length > 0 ? node.maxLen[0] : "-"}</span>
      </div>
      <div>
        <div class="text-(--text-muted)">Options</div>
        <div class="truncate text-(--text-secondary)">
          {node.options && node.options.length ? node.options.join(", ") : "-"}
        </div>
      </div>
    {/if}
  </div>
</OverlaySurface>
