<script lang="ts">
  import { onMount } from "svelte";
  import { Input } from "$lib/components/ui/input";
  import type { AutocompleteItem } from "./types";

  interface Props {
    value?: string | null;
    selectedLabel?: string | null;
    placeholder?: string;
    emptyText?: string;
    loadingText?: string;
    disabled?: boolean;
    searchItems: (query: string) => Promise<AutocompleteItem[]>;
    onValueChange: (item: AutocompleteItem | null) => void;
  }

  let {
    value = null,
    selectedLabel = null,
    placeholder = "Search...",
    emptyText = "No matches",
    loadingText = "Loading...",
    disabled = false,
    searchItems,
    onValueChange,
  }: Props = $props();

  let query = $state("");
  let open = $state(false);
  let loading = $state(false);
  let highlightedIndex = $state(0);
  let results = $state<AutocompleteItem[]>([]);
  let rootEl = $state<HTMLElement | null>(null);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let requestCounter = 0;

  onMount(() => {
    const onPointerDown = (event: PointerEvent): void => {
      if (!open || !rootEl) {
        return;
      }
      const target = event.target;
      if (target instanceof Node && rootEl.contains(target)) {
        return;
      }
      open = false;
    };

    document.addEventListener("pointerdown", onPointerDown, true);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown, true);
    };
  });

  $effect(() => {
    if (!open) {
      const next = selectedLabel ?? "";
      if (query !== next) {
        query = next;
      }
    }
  });

  $effect(() => {
    if (!open || disabled) {
      return;
    }

    if (debounceTimer) {
      clearTimeout(debounceTimer);
      debounceTimer = null;
    }

    debounceTimer = setTimeout(() => {
      debounceTimer = null;
      void loadResults(query);
    }, 220);

    return () => {
      if (debounceTimer) {
        clearTimeout(debounceTimer);
        debounceTimer = null;
      }
    };
  });

  async function loadResults(nextQuery: string): Promise<void> {
    const requestId = ++requestCounter;
    loading = true;

    try {
      const items = await searchItems(nextQuery.trim());
      if (requestId !== requestCounter) {
        return;
      }
      results = items;
      highlightedIndex = 0;
    } catch {
      if (requestId !== requestCounter) {
        return;
      }
      results = [];
      highlightedIndex = 0;
    } finally {
      if (requestId === requestCounter) {
        loading = false;
      }
    }
  }

  function applySelection(item: AutocompleteItem): void {
    query = item.label;
    open = false;
    onValueChange(item);
  }

  function clearSelection(): void {
    query = "";
    open = false;
    results = [];
    onValueChange(null);
  }

  function clampHighlight(nextIndex: number): void {
    if (results.length === 0) {
      highlightedIndex = 0;
      return;
    }

    if (nextIndex < 0) {
      highlightedIndex = results.length - 1;
      return;
    }
    if (nextIndex >= results.length) {
      highlightedIndex = 0;
      return;
    }
    highlightedIndex = nextIndex;
  }
</script>

<div class="relative" bind:this={rootEl}>
  <div class="flex items-center gap-1">
    <Input
      class="w-full text-xs"
      {disabled}
      value={query}
      {placeholder}
      onfocus={() => {
        open = true;
        void loadResults(query);
      }}
      oninput={(event) => {
        query = (event.currentTarget as HTMLInputElement).value;
        open = true;
      }}
      onkeydown={(event) => {
        if (!open && (event.key === "ArrowDown" || event.key === "ArrowUp")) {
          open = true;
          void loadResults(query);
          return;
        }

        if (!open) {
          return;
        }

        if (event.key === "ArrowDown") {
          event.preventDefault();
          clampHighlight(highlightedIndex + 1);
        } else if (event.key === "ArrowUp") {
          event.preventDefault();
          clampHighlight(highlightedIndex - 1);
        } else if (event.key === "Enter") {
          event.preventDefault();
          const candidate = results[highlightedIndex];
          if (candidate) {
            applySelection(candidate);
          }
        } else if (event.key === "Escape") {
          event.preventDefault();
          open = false;
        }
      }}
    />

    {#if value}
      <button
        type="button"
        class="rounded border border-border px-2 py-1 text-[10px] text-muted-foreground hover:bg-muted"
        onclick={clearSelection}
        title="Clear"
      >
        Clear
      </button>
    {/if}
  </div>

  {#if open && !disabled}
    <div class="absolute z-50 mt-1 max-h-56 w-full overflow-auto rounded-md border border-border bg-popover p-1 shadow-md">
      {#if loading}
        <div class="px-2 py-1.5 text-xs text-muted-foreground">{loadingText}</div>
      {:else if results.length === 0}
        <div class="px-2 py-1.5 text-xs text-muted-foreground">{emptyText}</div>
      {:else}
        {#each results as item, index (item.id)}
          <button
            type="button"
            class={`w-full rounded px-2 py-1.5 text-left text-xs ${index === highlightedIndex ? "bg-accent text-accent-foreground" : "text-foreground hover:bg-muted"}`}
            onmouseenter={() => {
              highlightedIndex = index;
            }}
            onmousedown={(event) => {
              event.preventDefault();
              applySelection(item);
            }}
          >
            <div class="truncate">{item.label}</div>
            {#if item.description}
              <div class="truncate text-[10px] opacity-70">{item.description}</div>
            {/if}
          </button>
        {/each}
      {/if}
    </div>
  {/if}
</div>
