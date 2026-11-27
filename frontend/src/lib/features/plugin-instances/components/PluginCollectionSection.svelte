<script lang="ts">
  import { slide } from "svelte/transition";
  import { ChevronDown, ChevronRight, Plus } from "lucide-svelte";
  import { DropdownMenu } from "bits-ui";
  import { Button } from "$lib/components/Button";
  import type { PluginCollection, PluginInstance, PluginInstanceState } from "../types";

  interface Props {
    collection: PluginCollection;
    instances: PluginInstance[];
    expanded: boolean;
    selectedInstanceId: string | null;
    onToggle: () => void;
    onCreate: (pluginType: string) => void;
    onSelect: (instance: PluginInstance) => void;
  }

  let {
    collection,
    instances,
    expanded,
    selectedInstanceId,
    onToggle,
    onCreate,
    onSelect,
  }: Props = $props();

  const STATE_DOT: Record<PluginInstanceState, string> = {
    running: "bg-emerald-500",
    error: "bg-destructive",
    stopped: "bg-amber-500",
    disabled: "bg-muted-foreground/40",
  };

  function pluginName(pluginType: string): string {
    return (
      collection.plugins.find((plugin) => plugin.plugin_type === pluginType)?.display_name ??
      pluginType
    );
  }
</script>

<section class="border-b border-border last:border-b-0">
  <div class="flex items-center justify-between px-4 py-2.5">
    <button
      type="button"
      class="flex items-center gap-1.5 text-sm font-semibold text-foreground"
      aria-expanded={expanded}
      onclick={onToggle}
    >
      {#if expanded}
        <ChevronDown class="size-4 text-muted-foreground" />
      {:else}
        <ChevronRight class="size-4 text-muted-foreground" />
      {/if}
      {collection.label}
    </button>
    <DropdownMenu.Root>
      <DropdownMenu.Trigger disabled={collection.plugins.length === 0}>
        {#snippet child({ props })}
          <Button
            {...props}
            variant="icon"
            icon={Plus}
            title={`New in ${collection.label}`}
            ariaLabel={`New in ${collection.label}`}
            disabled={collection.plugins.length === 0}
          />
        {/snippet}
      </DropdownMenu.Trigger>
      <DropdownMenu.Portal>
        <DropdownMenu.Content
          align="end"
          sideOffset={4}
          class="ring-foreground/10 bg-popover text-popover-foreground z-50 min-w-40 rounded-lg p-1 shadow-md ring-1 outline-none"
        >
          {#each collection.plugins as plugin (plugin.plugin_type)}
            <DropdownMenu.Item
              class="focus:bg-accent focus:text-accent-foreground flex min-h-7 cursor-default items-center rounded-md px-2 py-1 text-xs/relaxed outline-hidden select-none"
              onSelect={() => onCreate(plugin.plugin_type)}
            >
              {plugin.display_name}
            </DropdownMenu.Item>
          {/each}
        </DropdownMenu.Content>
      </DropdownMenu.Portal>
    </DropdownMenu.Root>
  </div>

  {#if expanded}
    <ul class="pb-2" transition:slide={{ duration: 150 }}>
      {#each instances as instance (instance.id)}
        <li>
          <button
            type="button"
            class={`flex w-full flex-col gap-0.5 px-4 py-2 pl-10 text-left hover:bg-muted/60 ${
              selectedInstanceId === instance.id ? "bg-muted" : ""
            }`}
            onclick={() => onSelect(instance)}
          >
            <span class="flex items-center gap-2">
              <span
                class={`size-2 shrink-0 rounded-full ${STATE_DOT[instance.status.state]}`}
                title={instance.status.state}
              ></span>
              <span class="truncate text-sm text-foreground">{instance.name}</span>
              <span class="ml-auto shrink-0 text-xs text-muted-foreground">
                {pluginName(instance.plugin_type)}
              </span>
            </span>
            {#if instance.status.last_error}
              <span class="truncate pl-4 text-xs text-destructive">
                {instance.status.last_error}
              </span>
            {/if}
          </button>
        </li>
      {:else}
        <li class="px-4 py-2 pl-10 text-xs text-muted-foreground">None yet</li>
      {/each}
    </ul>
  {/if}
</section>
