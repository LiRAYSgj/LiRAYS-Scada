<script lang="ts">
  import { onMount } from "svelte";
  import type { PlantAssetNodeData } from "$lib/features/graph/assets/types";

  interface Props {
    id: string;
    tagName: string;
    data: PlantAssetNodeData;
    selected?: boolean;
  }

  interface WidgetHostElement extends HTMLElement {
    widgetData?: PlantAssetNodeData;
    widgetSelected?: boolean;
  }

  let { id: _id, tagName, data, selected = false }: Props = $props();
  let host: WidgetHostElement | null = null;

  function syncElementProps(): void {
    if (!host) return;
    host.widgetData = data;
    host.widgetSelected = selected;
  }

  onMount(() => {
    syncElementProps();
  });

  $effect(() => {
    syncElementProps();
  });
</script>

<svelte:element this={tagName} bind:this={host} class="h-full w-full" />
