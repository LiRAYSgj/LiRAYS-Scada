<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { beforeNavigate, goto } from "$app/navigation";
  import { Button } from "$lib/components/Button";
  import * as Dialog from "$lib/components/ui/dialog";
  import PluginCollectionSection from "./PluginCollectionSection.svelte";
  import PluginInstanceEditor from "./PluginInstanceEditor.svelte";
  import {
    createPluginInstance,
    deletePluginInstance,
    listPluginCollections,
    listPluginInstances,
    updatePluginInstance,
  } from "../api/plugin-instances-api";
  import {
    buildConfigPatch,
    createFormValues,
    formValuesToConfig,
    parseJsonObject,
    validateFormValues,
  } from "../form-schema";
  import type {
    FormErrors,
    PluginCollection,
    PluginDescriptor,
    PluginInstance,
    PluginInstanceDraft,
  } from "../types";

  const STATUS_REFRESH_MS = 3_000;

  interface EditorTarget {
    collectionId: string;
    /** `null` while creating. */
    instanceId: string | null;
  }

  let collections = $state<PluginCollection[]>([]);
  let instancesByCollection = $state<Record<string, PluginInstance[]>>({});
  let collapsed = $state<Record<string, boolean>>({});
  let loading = $state(true);
  let loadError = $state("");
  let instancesRequestSerial = 0;
  let refreshTimer: ReturnType<typeof setInterval> | null = null;

  let target = $state<EditorTarget | null>(null);
  let draft = $state<PluginInstanceDraft | null>(null);
  let draftSnapshot = $state("");
  let errors = $state<FormErrors>({});
  let saveError = $state("");
  let saving = $state(false);

  let deleteDialogOpen = $state(false);
  let deleting = $state(false);
  let deleteError = $state("");

  let discardDialogOpen = $state(false);
  let pendingDiscardAction: (() => void) | null = null;
  let bypassLeaveGuard = false;

  const targetCollection = $derived(
    target ? (collections.find((item) => item.id === target?.collectionId) ?? null) : null,
  );
  const targetInstance = $derived(
    target?.instanceId && targetCollection
      ? ((instancesByCollection[targetCollection.id] ?? []).find(
          (item) => item.id === target?.instanceId,
        ) ?? null)
      : null,
  );
  const targetDescriptor = $derived(
    draft && targetCollection ? findDescriptor(targetCollection, draft.pluginType) : null,
  );
  const dirty = $derived(draft !== null && JSON.stringify(draft) !== draftSnapshot);

  function findDescriptor(
    collection: PluginCollection,
    pluginType: string,
  ): PluginDescriptor | null {
    return collection.plugins.find((plugin) => plugin.plugin_type === pluginType) ?? null;
  }

  function errorMessage(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  async function loadCollections(): Promise<void> {
    loading = true;
    loadError = "";
    try {
      collections = await listPluginCollections();
      await refreshInstances();
    } catch (error) {
      loadError = errorMessage(error);
    } finally {
      loading = false;
    }
  }

  async function refreshInstances(): Promise<void> {
    const serial = ++instancesRequestSerial;
    const entries = await Promise.all(
      collections.map(
        async (collection) =>
          [collection.id, await listPluginInstances(collection.base_path)] as const,
      ),
    );
    if (serial !== instancesRequestSerial) {
      return;
    }
    instancesByCollection = Object.fromEntries(entries);
  }

  function openDraft(
    collection: PluginCollection,
    instance: PluginInstance | null,
    pluginType: string,
  ): void {
    const descriptor = findDescriptor(collection, pluginType);
    const config = instance?.config ?? {};
    const next: PluginInstanceDraft = {
      pluginType,
      name: instance?.name ?? "",
      enabled: instance?.enabled ?? true,
      values: descriptor?.fields ? createFormValues(descriptor.fields, config) : {},
      rawConfig: descriptor?.fields ? "" : JSON.stringify(config, null, 2),
    };
    target = { collectionId: collection.id, instanceId: instance?.id ?? null };
    draft = next;
    draftSnapshot = JSON.stringify(next);
    errors = {};
    saveError = "";
  }

  function closeEditor(): void {
    target = null;
    draft = null;
    draftSnapshot = "";
    errors = {};
    saveError = "";
  }

  /** Runs `action` now, or after the user confirms discarding unsaved changes. */
  function guardUnsaved(action: () => void): void {
    if (!dirty) {
      action();
      return;
    }
    pendingDiscardAction = action;
    discardDialogOpen = true;
  }

  function confirmDiscard(): void {
    discardDialogOpen = false;
    const action = pendingDiscardAction;
    pendingDiscardAction = null;
    action?.();
  }

  function startCreate(collection: PluginCollection, pluginType: string): void {
    guardUnsaved(() => openDraft(collection, null, pluginType));
  }

  function selectInstance(collection: PluginCollection, instance: PluginInstance): void {
    if (target?.instanceId === instance.id) {
      return;
    }
    guardUnsaved(() => openDraft(collection, instance, instance.plugin_type));
  }

  async function save(): Promise<void> {
    if (!draft || !targetCollection || saving) {
      return;
    }
    const nextErrors: FormErrors = {};
    if (!draft.name.trim()) {
      nextErrors.name = "Required";
    }
    let config: Record<string, unknown> | null = null;
    const fields = targetDescriptor?.fields;
    if (fields) {
      Object.assign(
        nextErrors,
        validateFormValues(fields, draft.values, { keepEmptySecrets: targetInstance !== null }),
      );
      config = formValuesToConfig(fields, draft.values);
    } else {
      config = parseJsonObject(draft.rawConfig);
      if (!config) {
        nextErrors.config = "Must be a JSON object";
      }
    }
    errors = nextErrors;
    if (Object.keys(nextErrors).length > 0 || !config) {
      return;
    }

    saving = true;
    saveError = "";
    const collection = targetCollection;
    try {
      const saved = targetInstance
        ? await updatePluginInstance(collection.base_path, targetInstance.id, {
            name: draft.name.trim(),
            enabled: draft.enabled,
            config: buildConfigPatch(targetInstance.config, config),
          })
        : await createPluginInstance(collection.base_path, {
            name: draft.name.trim(),
            plugin_type: draft.pluginType,
            enabled: draft.enabled,
            config,
          });
      const instances = instancesByCollection[collection.id] ?? [];
      instancesByCollection = {
        ...instancesByCollection,
        [collection.id]: instances.some((item) => item.id === saved.id)
          ? instances.map((item) => (item.id === saved.id ? saved : item))
          : [...instances, saved],
      };
      openDraft(collection, saved, saved.plugin_type);
      void refreshInstances().catch(() => undefined);
    } catch (error) {
      saveError = errorMessage(error);
    } finally {
      saving = false;
    }
  }

  async function confirmDelete(): Promise<void> {
    if (!targetCollection || !targetInstance || deleting) {
      return;
    }
    deleting = true;
    deleteError = "";
    const collection = targetCollection;
    const instanceId = targetInstance.id;
    try {
      await deletePluginInstance(collection.base_path, instanceId);
      instancesByCollection = {
        ...instancesByCollection,
        [collection.id]: (instancesByCollection[collection.id] ?? []).filter(
          (item) => item.id !== instanceId,
        ),
      };
      deleteDialogOpen = false;
      closeEditor();
    } catch (error) {
      deleteError = errorMessage(error);
    } finally {
      deleting = false;
    }
  }

  beforeNavigate(({ to, cancel, type }) => {
    if (bypassLeaveGuard || !dirty || !to || type === "leave") {
      return;
    }
    cancel();
    const href = `${to.url.pathname}${to.url.search}${to.url.hash}`;
    guardUnsaved(() => {
      bypassLeaveGuard = true;
      void goto(href).finally(() => {
        bypassLeaveGuard = false;
      });
    });
  });

  onMount(() => {
    void loadCollections();
    refreshTimer = setInterval(() => {
      if (document.hidden || loading || collections.length === 0) {
        return;
      }
      void refreshInstances().catch(() => undefined);
    }, STATUS_REFRESH_MS);
  });

  onDestroy(() => {
    if (refreshTimer) {
      clearInterval(refreshTimer);
    }
    instancesRequestSerial += 1;
  });
</script>

<div class="flex h-full min-h-0">
  <div class="flex min-h-0 min-w-0 flex-1 flex-col">
    <div class="border-b border-border px-4 py-3">
      <h2 class="text-xl font-semibold text-foreground">Integrations</h2>
    </div>
    <div class="min-h-0 flex-1 overflow-auto">
      {#if loading}
        <p class="px-4 py-6 text-sm text-muted-foreground">Loading…</p>
      {:else if loadError}
        <div class="space-y-3 px-4 py-6">
          <p class="text-sm text-destructive">{loadError}</p>
          <Button variant="outline-muted" size="sm" label="Retry" onclick={() => void loadCollections()} />
        </div>
      {:else}
        {#each collections as collection (collection.id)}
          <PluginCollectionSection
            {collection}
            instances={instancesByCollection[collection.id] ?? []}
            expanded={!collapsed[collection.id]}
            selectedInstanceId={target?.collectionId === collection.id ? target.instanceId : null}
            onToggle={() => (collapsed[collection.id] = !collapsed[collection.id])}
            onCreate={(pluginType) => startCreate(collection, pluginType)}
            onSelect={(instance) => selectInstance(collection, instance)}
          />
        {/each}
      {/if}
    </div>
  </div>

  {#if draft && targetCollection}
    <aside class="flex h-full w-[42%] min-w-[340px] max-w-[520px] flex-col border-l border-border">
      <PluginInstanceEditor
        collection={targetCollection}
        descriptor={targetDescriptor}
        instance={targetInstance}
        bind:draft
        {errors}
        {saveError}
        {saving}
        {dirty}
        onSave={() => void save()}
        onClose={() => guardUnsaved(closeEditor)}
        onDelete={() => {
          deleteError = "";
          deleteDialogOpen = true;
        }}
      />
    </aside>
  {/if}
</div>

<Dialog.Root bind:open={deleteDialogOpen}>
  <Dialog.Content class="max-w-[420px]" showCloseButton={false}>
    <form
      class="flex flex-col gap-4"
      onsubmit={(event) => {
        event.preventDefault();
        void confirmDelete();
      }}
    >
      <Dialog.Header>
        <Dialog.Title>Confirm removal</Dialog.Title>
        <Dialog.Description>
          Remove "{targetInstance?.name}"? Its namespace variables are kept.
        </Dialog.Description>
        {#if deleteError}
          <p class="text-destructive text-xs/relaxed">{deleteError}</p>
        {/if}
      </Dialog.Header>
      <Dialog.Footer class="border-border border-t pt-4">
        <Button
          variant="outline-muted"
          label="Cancel"
          disabled={deleting}
          onclick={() => (deleteDialogOpen = false)}
        />
        <Button
          type="submit"
          variant="filled-warn"
          label="Remove"
          loadingLabel="Removing..."
          loading={deleting}
          disabled={deleting || !targetInstance}
        />
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>

<Dialog.Root
  bind:open={discardDialogOpen}
  onOpenChange={(open) => {
    if (!open) {
      pendingDiscardAction = null;
    }
  }}
>
  <Dialog.Content class="max-w-[420px]" showCloseButton={false}>
    <Dialog.Header>
      <Dialog.Title>Discard changes?</Dialog.Title>
      <Dialog.Description>Unsaved changes will be lost.</Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer class="border-border border-t pt-4">
      <Button
        variant="outline-muted"
        label="Keep editing"
        onclick={() => {
          discardDialogOpen = false;
          pendingDiscardAction = null;
        }}
      />
      <Button variant="filled-warn" label="Discard" onclick={confirmDiscard} />
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
