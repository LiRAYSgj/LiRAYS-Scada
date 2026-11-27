<script lang="ts">
  import DynamicFields from "./DynamicFields.svelte";
  import { Plus, X } from "lucide-svelte";
  import { Button } from "$lib/components/Button";
  import { Input } from "$lib/components/ui/input";
  import { Textarea } from "$lib/components/ui/textarea";
  import { Checkbox } from "$lib/components/ui/checkbox";
  import { TagsInput } from "$lib/components/ui/tags-input";
  import * as Select from "$lib/components/ui/select";
  import { createFormValues, isFieldVisible } from "../form-schema";
  import { folderPattern, hasTreePath, readTreePath } from "../tree-path-drag";
  import type { FormErrors, FormField, FormValues } from "../types";

  interface Props {
    fields: FormField[];
    values: FormValues;
    errors: FormErrors;
    /** Dotted path of `values` inside the whole form, used to look up errors. */
    path?: string;
    keepEmptySecrets?: boolean;
    disabled?: boolean;
  }

  let {
    fields,
    values = $bindable(),
    errors,
    path = "",
    keepEmptySecrets = false,
    disabled = false,
  }: Props = $props();

  function fieldPath(field: FormField): string {
    return path ? `${path}.${field.key}` : field.key;
  }

  function asRecord(value: unknown): FormValues {
    return typeof value === "object" && value !== null && !Array.isArray(value)
      ? (value as FormValues)
      : {};
  }

  function asItems(value: unknown): FormValues[] {
    return Array.isArray(value) ? (value as FormValues[]) : [];
  }

  function asStrings(value: unknown): string[] {
    return Array.isArray(value) ? value.map(String) : [];
  }

  function setValue(field: FormField, value: unknown): void {
    values[field.key] = value;
  }

  function addListItem(field: FormField): void {
    setValue(field, [...asItems(values[field.key]), createFormValues(field.fields ?? [])]);
  }

  function removeListItem(field: FormField, index: number): void {
    setValue(
      field,
      asItems(values[field.key]).filter((_, itemIndex) => itemIndex !== index),
    );
  }

  function allowTreePathDrop(event: DragEvent): void {
    if (disabled || !hasTreePath(event)) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = "copy";
    }
  }

  function dropTreePath(event: DragEvent, field: FormField): void {
    const drop = readTreePath(event);
    if (disabled || !drop) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    const entry = drop.isFolder && field.folder_as_pattern ? folderPattern(drop.path) : drop.path;
    if (field.type === "tag_list") {
      const current = asStrings(values[field.key]);
      if (!current.includes(entry)) {
        setValue(field, [...current, entry]);
      }
      return;
    }
    setValue(field, entry);
  }

  function selectLabel(field: FormField, value: unknown): string {
    const option = field.options?.find((item) => item.value === value);
    return option?.label ?? "Select…";
  }
</script>

<div class="space-y-3">
  {#each fields as field (field.key)}
    {#if isFieldVisible(field, values)}
      {@const error = errors[fieldPath(field)]}
      <div class="space-y-1.5">
        {#if field.type !== "boolean"}
          <p class="text-xs font-medium text-foreground">
            {field.label}{#if field.required}<span class="text-muted-foreground"> *</span>{/if}
          </p>
        {/if}

        {#if field.type === "group"}
          <div class="space-y-3 border-l border-border pl-3">
            <DynamicFields
              fields={field.fields ?? []}
              bind:values={() => asRecord(values[field.key]), (next) => setValue(field, next)}
              {errors}
              path={fieldPath(field)}
              {keepEmptySecrets}
              {disabled}
            />
          </div>
        {:else if field.type === "list"}
          {@const items = asItems(values[field.key])}
          <div class="space-y-2">
            {#each items as item, index (index)}
              <div class="relative rounded-md border border-border p-3 pr-10">
                <Button
                  variant="icon"
                  icon={X}
                  class="absolute right-2 top-2"
                  title="Remove"
                  ariaLabel="Remove"
                  {disabled}
                  onclick={() => removeListItem(field, index)}
                />
                <DynamicFields
                  fields={field.fields ?? []}
                  bind:values={
                    () => item,
                    (next) => {
                      const updated = [...asItems(values[field.key])];
                      updated[index] = next;
                      setValue(field, updated);
                    }
                  }
                  {errors}
                  path={`${fieldPath(field)}.${index}`}
                  {keepEmptySecrets}
                  {disabled}
                />
              </div>
            {/each}
            <Button
              variant="outline-muted"
              size="sm"
              icon={Plus}
              label="Add"
              {disabled}
              onclick={() => addListItem(field)}
            />
          </div>
        {:else if field.type === "boolean"}
          <label class="flex items-center gap-2 text-xs text-foreground">
            <Checkbox
              checked={Boolean(values[field.key])}
              onCheckedChange={(checked) => setValue(field, checked)}
              {disabled}
            />
            <span>{field.label}</span>
          </label>
        {:else if field.type === "select"}
          <Select.Root
            type="single"
            value={String(values[field.key] ?? "")}
            onValueChange={(value) => setValue(field, value)}
            {disabled}
          >
            <Select.Trigger class="h-8 w-full text-xs" aria-invalid={Boolean(error)}>
              {selectLabel(field, values[field.key])}
            </Select.Trigger>
            <Select.Content class="border border-border bg-card">
              <Select.Group>
                {#each field.options ?? [] as option (option.value)}
                  <Select.Item value={option.value} label={option.label} />
                {/each}
              </Select.Group>
            </Select.Content>
          </Select.Root>
        {:else if field.type === "number"}
          <Input
            class="h-8 text-xs"
            type="number"
            min={field.min}
            max={field.max}
            step={field.step ?? "any"}
            value={values[field.key] === undefined || values[field.key] === null
              ? ""
              : String(values[field.key])}
            placeholder={field.placeholder ?? ""}
            aria-invalid={Boolean(error)}
            {disabled}
            oninput={(event) => {
              const raw = (event.currentTarget as HTMLInputElement).value.trim();
              setValue(field, raw === "" ? undefined : Number(raw));
            }}
          />
        {:else if field.type === "tag_list"}
          <div
            role="group"
            aria-label={field.label}
            ondragover={allowTreePathDrop}
            ondrop={(event) => dropTreePath(event, field)}
          >
            <TagsInput
              class="text-xs"
              value={asStrings(values[field.key])}
              onValueChange={(next) => setValue(field, next)}
              placeholder={field.placeholder ?? "Type a path or drop a variable"}
              aria-invalid={Boolean(error)}
              {disabled}
            />
          </div>
        {:else if field.type === "textarea" || field.type === "json"}
          <Textarea
            class={field.type === "json" ? "min-h-24 font-mono text-xs" : "min-h-20 text-xs"}
            value={String(values[field.key] ?? "")}
            placeholder={field.placeholder ?? ""}
            aria-invalid={Boolean(error)}
            {disabled}
            oninput={(event) =>
              setValue(field, (event.currentTarget as HTMLTextAreaElement).value)}
          />
        {:else}
          <Input
            class="h-8 text-xs"
            type={field.type === "secret" ? "password" : "text"}
            autocomplete={field.type === "secret" ? "new-password" : "off"}
            value={String(values[field.key] ?? "")}
            placeholder={field.type === "secret" && keepEmptySecrets
              ? "Unchanged"
              : field.type === "tag"
                ? (field.placeholder ?? "Type a path or drop a variable")
                : (field.placeholder ?? "")}
            aria-invalid={Boolean(error)}
            {disabled}
            ondragover={field.type === "tag" ? allowTreePathDrop : undefined}
            ondrop={field.type === "tag" ? (event) => dropTreePath(event, field) : undefined}
            oninput={(event) => setValue(field, (event.currentTarget as HTMLInputElement).value)}
          />
        {/if}

        {#if error}
          <p class="text-[11px] text-destructive">{error}</p>
        {:else if field.description}
          <p class="text-[11px] text-muted-foreground">{field.description}</p>
        {/if}
      </div>
    {/if}
  {/each}
</div>
