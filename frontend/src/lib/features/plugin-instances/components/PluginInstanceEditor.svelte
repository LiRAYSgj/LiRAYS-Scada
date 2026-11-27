<script lang="ts">
  import { Trash2, X } from "lucide-svelte";
  import { Button } from "$lib/components/Button";
  import { Input } from "$lib/components/ui/input";
  import { Textarea } from "$lib/components/ui/textarea";
  import { Checkbox } from "$lib/components/ui/checkbox";
  import DynamicFields from "./DynamicFields.svelte";
  import type {
    FormErrors,
    PluginCollection,
    PluginDescriptor,
    PluginInstance,
    PluginInstanceDraft,
  } from "../types";

  interface Props {
    collection: PluginCollection;
    descriptor: PluginDescriptor | null;
    /** `null` while creating. */
    instance: PluginInstance | null;
    draft: PluginInstanceDraft;
    errors: FormErrors;
    saveError: string;
    saving: boolean;
    dirty: boolean;
    onSave: () => void;
    onClose: () => void;
    onDelete: () => void;
  }

  let {
    collection,
    descriptor,
    instance,
    draft = $bindable(),
    errors,
    saveError,
    saving,
    dirty,
    onSave,
    onClose,
    onDelete,
  }: Props = $props();

  const title = $derived(
    instance ? instance.name : `New ${descriptor?.display_name ?? collection.label}`,
  );
</script>

<form
  class="flex h-full min-h-0 flex-col"
  onsubmit={(event) => {
    event.preventDefault();
    onSave();
  }}
>
  <div class="flex items-start justify-between gap-2 border-b border-border px-4 py-3">
    <div class="min-w-0">
      <h3 class="truncate text-sm font-semibold text-foreground">{title}</h3>
      <p class="truncate text-xs text-muted-foreground">
        {descriptor?.display_name ?? draft.pluginType}{instance
          ? ` · ${instance.status.state}`
          : ""}
      </p>
    </div>
    <Button variant="icon" icon={X} title="Close" ariaLabel="Close" onclick={onClose} />
  </div>

  <div class="min-h-0 flex-1 space-y-3 overflow-auto px-4 py-3">
    {#if instance?.status.last_error}
      <p class="rounded-md border border-destructive/40 px-3 py-2 text-xs text-destructive">
        {instance.status.last_error}
      </p>
    {/if}

    {#if descriptor?.description}
      <p class="text-xs text-muted-foreground">{descriptor.description}</p>
    {/if}

    <div class="space-y-1.5">
      <p class="text-xs font-medium text-foreground">
        Name<span class="text-muted-foreground"> *</span>
      </p>
      <Input
        class="h-8 text-xs"
        bind:value={draft.name}
        aria-invalid={Boolean(errors.name)}
        disabled={saving}
      />
      {#if errors.name}
        <p class="text-[11px] text-destructive">{errors.name}</p>
      {/if}
    </div>

    <label class="flex items-center gap-2 text-xs text-foreground">
      <Checkbox bind:checked={draft.enabled} disabled={saving} />
      <span>Enabled</span>
    </label>

    {#if descriptor?.fields}
      <DynamicFields
        fields={descriptor.fields}
        bind:values={draft.values}
        {errors}
        keepEmptySecrets={instance !== null}
        disabled={saving}
      />
    {:else}
      <div class="space-y-1.5">
        <p class="text-xs font-medium text-foreground">Configuration (JSON)</p>
        <Textarea
          class="min-h-48 font-mono text-xs"
          bind:value={draft.rawConfig}
          aria-invalid={Boolean(errors.config)}
          disabled={saving}
        />
        {#if errors.config}
          <p class="text-[11px] text-destructive">{errors.config}</p>
        {/if}
      </div>
    {/if}
  </div>

  <div class="flex items-center gap-2 border-t border-border px-4 py-3">
    {#if instance}
      <Button
        variant="icon"
        icon={Trash2}
        title="Delete"
        ariaLabel="Delete"
        class="border-destructive/45 text-destructive hover:border-destructive/70 hover:bg-destructive/12 hover:text-destructive"
        disabled={saving}
        onclick={onDelete}
      />
    {/if}
    {#if saveError}
      <p class="min-w-0 flex-1 truncate text-xs text-destructive" title={saveError}>
        {saveError}
      </p>
    {:else}
      <span class="flex-1"></span>
    {/if}
    <Button
      type="submit"
      variant="filled-accent"
      size="sm"
      label="Save"
      loading={saving}
      loadingLabel="Saving…"
      disabled={saving || (instance !== null && !dirty)}
    />
  </div>
</form>
