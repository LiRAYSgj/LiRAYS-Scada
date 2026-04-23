<script lang="ts">
  import { Button } from "$lib/components/Button";
  import * as Card from "$lib/components/ui/card";
  import { Input } from "$lib/components/ui/input";
  import * as Select from "$lib/components/ui/select";
  import {
    ChevronsLeft,
    ChevronLeft,
    ChevronRight,
    ChevronsRight,
    Loader2,
    Pencil,
    Plus,
    Search,
    Trash2,
  } from "lucide-svelte";
  import type { ScadaView } from "../types";
  import { VIEWS_TABLE_COLUMNS } from "../table/views-table-columns";
  import {
    DEFAULT_VIEWS_TABLE_STATE,
    type ViewsTableState,
  } from "../table/views-table-state";

  interface Props {
    views: ScadaView[];
    total: number;
    errorMessage?: string;
    tableState: ViewsTableState;
    loading?: boolean;
    createLoading?: boolean;
    busyId?: string | null;
    searchValue?: string;
    onCreate: () => void;
    onEdit: (view: ScadaView) => void;
    onRemove: (view: ScadaView) => void;
    onInlineUpdate: (
      view: ScadaView,
      changes: { name?: string; description?: string },
    ) => Promise<boolean>;
    onSearchChange: (value: string) => void;
    onTableStateChange: (next: ViewsTableState) => void;
  }

  let {
    views,
    total,
    errorMessage = "",
    tableState,
    loading = false,
    createLoading = false,
    busyId = null,
    searchValue = "",
    onCreate,
    onEdit,
    onRemove,
    onInlineUpdate,
    onSearchChange,
    onTableStateChange,
  }: Props = $props();

  let pageSizeValue = $state(String(DEFAULT_VIEWS_TABLE_STATE.pageSize));
  let editingViewId = $state<string | null>(null);
  let editingField = $state<"name" | "description" | null>(null);
  let editingValue = $state("");
  let editingOriginalValue = $state("");
  let editingInputEl = $state<HTMLInputElement | null>(null);
  let inlineSubmitting = $state(false);
  let inlineSubmittingViewId = $state<string | null>(null);
  let inlineSubmittingField = $state<"name" | "description" | null>(null);

  $effect(() => {
    pageSizeValue = String(tableState.pageSize);
  });

  const pageCount = $derived(
    Math.max(1, Math.ceil(total / tableState.pageSize)),
  );
  const currentPageIndex = $derived(
    Math.min(tableState.pageIndex, pageCount - 1),
  );

  const rangeStart = $derived(
    total === 0 ? 0 : currentPageIndex * tableState.pageSize + 1,
  );
  const rangeEnd = $derived(
    total === 0
      ? 0
      : Math.min(total, currentPageIndex * tableState.pageSize + views.length),
  );

  function formatTs(unixTs: number): string {
    return new Date(unixTs * 1000).toLocaleString();
  }

  function startInlineEdit(view: ScadaView, field: "name" | "description"): void {
    if (inlineSubmitting || busyId === view.id) {
      return;
    }
    editingViewId = view.id;
    editingField = field;
    editingValue = field === "name" ? view.name : view.description;
    editingOriginalValue = editingValue;
    setTimeout(() => {
      editingInputEl?.focus();
      editingInputEl?.select();
    }, 0);
  }

  function cancelInlineEdit(): void {
    editingViewId = null;
    editingField = null;
    editingValue = "";
    editingOriginalValue = "";
    inlineSubmitting = false;
    inlineSubmittingViewId = null;
    inlineSubmittingField = null;
  }

  async function commitInlineEdit(): Promise<void> {
    if (!editingViewId || !editingField || inlineSubmitting) {
      return;
    }
    const view = views.find((item) => item.id === editingViewId);
    if (!view) {
      cancelInlineEdit();
      return;
    }

    const nextValue = editingValue.trim();
    const currentValue =
      editingField === "name" ? view.name.trim() : view.description.trim();
    if (nextValue === currentValue) {
      cancelInlineEdit();
      return;
    }

    inlineSubmitting = true;
    inlineSubmittingViewId = view.id;
    inlineSubmittingField = editingField;
    let success = false;
    try {
      success = await onInlineUpdate(
        view,
        editingField === "name"
          ? { name: nextValue }
          : { description: nextValue },
      );
    } catch {
      success = false;
    }
    inlineSubmitting = false;

    if (success) {
      cancelInlineEdit();
      return;
    }

    // On any API/connection error, discard unsaved local edits and return to persisted value.
    editingValue = editingOriginalValue;
    cancelInlineEdit();
  }

  $effect(() => {
    if (!editingViewId) {
      return;
    }
    const stillVisible = views.some((view) => view.id === editingViewId);
    if (!stillVisible) {
      cancelInlineEdit();
    }
  });

  function toggleSort(column: "name" | "updated_at" | "is_entry_point"): void {
    if (tableState.sortBy === column) {
      onTableStateChange({
        ...tableState,
        sortDirection: tableState.sortDirection === "asc" ? "desc" : "asc",
        pageIndex: 0,
      });
      return;
    }
    onTableStateChange({
      ...tableState,
      sortBy: column,
      sortDirection: column === "name" ? "asc" : "desc",
      pageIndex: 0,
    });
  }

  function setPageSize(nextSize: number): void {
    pageSizeValue = String(nextSize);
    onTableStateChange({
      ...tableState,
      pageSize: nextSize,
      pageIndex: 0,
    });
  }

  function setPageIndex(index: number): void {
    onTableStateChange({
      ...tableState,
      pageIndex: Math.max(0, Math.min(index, pageCount - 1)),
    });
  }
</script>

<Card.Root class="flex h-full flex-col rounded-md border-border">
  <Card.Header class="border-b border-border pb-4">
    <div class="flex items-start justify-between gap-4">
      <div class="space-y-1">
        <Card.Title class="text-xl font-semibold">Views</Card.Title>
        <Card.Description>Manage runtime views.</Card.Description>
      </div>
      <div class="flex items-center gap-2">
        <Button
          variant="outline-accent"
          icon={Plus}
          label="Create"
          loading={createLoading}
          loadingLabel="Creating..."
          disabled={createLoading}
          title="Create"
          ariaLabel="Create"
          onclick={onCreate}
        />
        <div class="relative">
          <Input
            id="views-search"
            name="views-search"
            class="h-9 w-[240px] pr-9"
            type="search"
            autocomplete="off"
            aria-label="Search views"
            placeholder="Search by name or description"
            value={searchValue}
            oninput={(event) =>
              onSearchChange((event.currentTarget as HTMLInputElement).value)}
          />
          <Search
            class="pointer-events-none absolute right-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground"
          />
        </div>
      </div>
    </div>
  </Card.Header>

  <Card.Content class="flex min-h-0 flex-1 flex-col px-0 pb-0">
    <div class="min-h-0 flex-1 overflow-auto">
      <table class="w-full table-fixed border-collapse">
        <thead class="sticky top-0 z-10 bg-card">
          <tr class="border-b border-border text-left text-xs uppercase text-muted-foreground">
            {#each VIEWS_TABLE_COLUMNS as column}
              <th class={`px-6 py-3 font-medium ${column.className ?? ""}`}>
                {#if column.id === "view"}
                  <button
                    type="button"
                    class="inline-flex items-center gap-1 hover:text-foreground uppercase"
                    onclick={() => toggleSort("name")}
                  >
                    <span>{column.title}</span>
                  </button>
                {:else if column.id === "updated"}
                  <button
                    type="button"
                    class="inline-flex items-center gap-1 hover:text-foreground uppercase"
                    onclick={() => toggleSort("updated_at")}
                  >
                    <span>{column.title}</span>
                  </button>
                {:else if column.id === "entry"}
                  <button
                    type="button"
                    class="inline-flex items-center gap-1 hover:text-foreground uppercase"
                    onclick={() => toggleSort("is_entry_point")}
                  >
                    <span>{column.title}</span>
                  </button>
                {:else}
                  <span>{column.title}</span>
                {/if}
              </th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#if loading}
            <tr>
              <td colspan={VIEWS_TABLE_COLUMNS.length} class="px-6 py-6 text-sm text-muted-foreground">
                Loading views...
              </td>
            </tr>
          {:else if errorMessage}
            <tr>
              <td colspan={VIEWS_TABLE_COLUMNS.length} class="px-6 py-6 text-sm text-destructive">
                Failed to load views: {errorMessage}
              </td>
            </tr>
          {:else if views.length === 0}
            <tr>
              <td colspan={VIEWS_TABLE_COLUMNS.length} class="px-6 py-6 text-sm text-muted-foreground">
                No views available.
              </td>
            </tr>
          {:else}
            {#each views as view (view.id)}
              <tr class="border-b border-border/70">
                <td class="px-6 py-3 align-top">
                  {#if editingViewId === view.id && editingField === "name"}
                    <div class="relative max-w-[340px]">
                      <Input
                        id={`view-name-${view.id}`}
                        name={`view-name-${view.id}`}
                        bind:ref={editingInputEl}
                        class="block h-7 max-w-[340px] border-primary/60 pr-8 text-sm font-semibold"
                        value={editingValue}
                        disabled={inlineSubmitting || busyId === view.id}
                        oninput={(event) =>
                          (editingValue = (event.currentTarget as HTMLInputElement).value)}
                        onkeydown={(event) => {
                          if (event.key === "Enter") {
                            event.preventDefault();
                            void commitInlineEdit();
                          } else if (event.key === "Escape") {
                            event.preventDefault();
                            cancelInlineEdit();
                          }
                        }}
                        onblur={() => void commitInlineEdit()}
                      />
                      {#if inlineSubmitting && inlineSubmittingViewId === view.id && inlineSubmittingField === "name"}
                        <Loader2
                          class="pointer-events-none absolute right-2 top-1/2 size-3.5 -translate-y-1/2 animate-spin text-muted-foreground"
                        />
                      {/if}
                    </div>
                  {:else}
                    <button
                      type="button"
                      class="block cursor-text text-left text-sm font-semibold text-foreground hover:text-primary"
                      ondblclick={() => startInlineEdit(view, "name")}
                      title="Double click to edit name"
                    >
                      {view.name}
                    </button>
                  {/if}
                  {#if editingViewId === view.id && editingField === "description"}
                    <div class="relative mt-1 max-w-[420px]">
                      <Input
                        id={`view-description-${view.id}`}
                        name={`view-description-${view.id}`}
                        bind:ref={editingInputEl}
                        class="block h-7 max-w-[420px] border-primary/60 pr-8 text-xs"
                        value={editingValue}
                        disabled={inlineSubmitting || busyId === view.id}
                        placeholder="Description"
                        oninput={(event) =>
                          (editingValue = (event.currentTarget as HTMLInputElement).value)}
                        onkeydown={(event) => {
                          if (event.key === "Enter") {
                            event.preventDefault();
                            void commitInlineEdit();
                          } else if (event.key === "Escape") {
                            event.preventDefault();
                            cancelInlineEdit();
                          }
                        }}
                        onblur={() => void commitInlineEdit()}
                      />
                      {#if inlineSubmitting && inlineSubmittingViewId === view.id && inlineSubmittingField === "description"}
                        <Loader2
                          class="pointer-events-none absolute right-2 top-1/2 size-3.5 -translate-y-1/2 animate-spin text-muted-foreground"
                        />
                      {/if}
                    </div>
                  {:else}
                    <button
                      type="button"
                      class={`mt-0.5 block cursor-text text-left text-xs hover:text-primary ${
                        view.description.trim()
                          ? "text-muted-foreground"
                          : "text-muted-foreground/70 italic"
                      }`}
                      ondblclick={() => startInlineEdit(view, "description")}
                      title="Double click to edit description"
                    >
                      {view.description.trim() || "Add description"}
                    </button>
                  {/if}
                </td>
                <td class="px-6 py-3 align-top">
                  <div class="text-xs text-muted-foreground">{formatTs(view.updated_at)}</div>
                </td>
                <td class="px-6 py-3 align-top">
                  {#if view.is_entry_point}
                    <span
                      class="inline-flex rounded-full border border-primary/35 bg-primary/12 px-2 py-0.5 text-xs font-medium text-foreground"
                    >
                      Entry point
                    </span>
                  {:else}
                    <span class="text-xs text-muted-foreground">-</span>
                  {/if}
                </td>
                <td class="px-6 py-3 align-top">
                  <div class="flex justify-end gap-1">
                    <Button
                      variant="icon"
                      icon={Pencil}
                      label=""
                      title="Edit view"
                      ariaLabel="Edit view"
                      disabled={busyId === view.id}
                      onclick={() => onEdit(view)}
                    />
                    <Button
                      variant="icon"
                      icon={Trash2}
                      label=""
                      title="Remove view"
                      ariaLabel="Remove view"
                      class="border-destructive/45 text-destructive hover:border-destructive/70 hover:bg-destructive/12 hover:text-destructive"
                      disabled={busyId === view.id || total <= 1 || view.is_entry_point}
                      onclick={() => onRemove(view)}
                    />
                  </div>
                </td>
              </tr>
            {/each}
          {/if}
        </tbody>
      </table>
    </div>

    <div class="flex items-center justify-between border-t border-border px-6 py-3 text-xs text-muted-foreground">
      <div class="flex items-center gap-2">
        <span>Items per page:</span>
        <Select.Root
          type="single"
          bind:value={pageSizeValue}
          onValueChange={() => setPageSize(Number(pageSizeValue))}
        >
          <Select.Trigger class="h-7 w-[78px] text-xs">{pageSizeValue}</Select.Trigger>
          <Select.Content portalProps={{ disabled: true }} class="border border-border bg-card">
            <Select.Group>
              <Select.Item value="5" label="5" />
              <Select.Item value="10" label="10" />
              <Select.Item value="20" label="20" />
              <Select.Item value="50" label="50" />
            </Select.Group>
          </Select.Content>
        </Select.Root>
      </div>

      <div class="flex items-center gap-3">
        <span>{rangeStart} - {rangeEnd} of {total}</span>
        <div class="flex items-center gap-1">
          <Button
            variant="icon"
            icon={ChevronsLeft}
            label=""
            title="First page"
            ariaLabel="First page"
            disabled={currentPageIndex <= 0}
            onclick={() => setPageIndex(0)}
          />
          <Button
            variant="icon"
            icon={ChevronLeft}
            label=""
            title="Previous page"
            ariaLabel="Previous page"
            disabled={currentPageIndex <= 0}
            onclick={() => setPageIndex(currentPageIndex - 1)}
          />
          <Button
            variant="icon"
            icon={ChevronRight}
            label=""
            title="Next page"
            ariaLabel="Next page"
            disabled={currentPageIndex >= pageCount - 1}
            onclick={() => setPageIndex(currentPageIndex + 1)}
          />
          <Button
            variant="icon"
            icon={ChevronsRight}
            label=""
            title="Last page"
            ariaLabel="Last page"
            disabled={currentPageIndex >= pageCount - 1}
            onclick={() => setPageIndex(pageCount - 1)}
          />
        </div>
      </div>
    </div>
  </Card.Content>
</Card.Root>
