<script lang="ts">
  import { defaults, superForm } from "sveltekit-superforms/client";
  import { zod4, zod4Client } from "sveltekit-superforms/adapters";
  import { editTreeMetaSchema, toEditMetaPayload } from "$lib/forms/tree-schemas";
  import type { TreeNode } from "../types";
  import { Button } from "$lib/components/Button";
  import { Input } from "$lib/components/ui/input";
  import { NumberField } from "$lib/components/ui/number-field";
  import { TagsInput } from "$lib/components/ui/tags-input";

  interface Props {
    open?: boolean;
    connected: boolean;
    node: TreeNode | null;
    onEditMeta: (input: {
      varId: string;
      unit?: string;
      min?: number;
      max?: number;
      options?: string[];
      maxLen?: number;
    }) => Promise<void>;
    onRefreshParent: (parentId: string | null) => Promise<void>;
  }

  type EditFieldKey =
    | "varId"
    | "dataType"
    | "unit"
    | "min"
    | "max"
    | "options"
    | "maxLen";

  let {
    open = $bindable(false),
    connected,
    node,
    onEditMeta,
    onRefreshParent,
  }: Props = $props();

  const editDialogForm = superForm(
    defaults(
      {
        varId: "",
        dataType: "",
        unit: "",
        min: undefined,
        max: undefined,
        options: "",
        maxLen: undefined,
      },
      zod4(editTreeMetaSchema),
    ),
    {
      SPA: true,
      validators: zod4Client(editTreeMetaSchema),
      validationMethod: "submit-only",
    },
  );

  let dialogEl: HTMLDialogElement | null = null;
  let editVarId = $state<string | null>(null);
  let editUnit = $state("");
  let editMin = $state<number | undefined>(undefined);
  let editMax = $state<number | undefined>(undefined);
  let editOptionTags = $state<string[]>([]);
  let editMaxLen = $state<number | undefined>(undefined);
  let editDataType = $state<string>("");
  let editError = $state("");
  let editSubmitting = $state(false);
  let editTouched = $state<Partial<Record<EditFieldKey, boolean>>>({});

  function serializeOptionTags(tags: string[]): string {
    return tags.join(",");
  }

  function hydrateFromNode(sourceNode: TreeNode | null): void {
    const nextVarId = sourceNode?.id ?? null;
    const nextUnit = sourceNode?.unit ?? "";
    const nextMin = sourceNode?.min;
    const nextMax = sourceNode?.max;
    const nextOptionTags = sourceNode?.options ? [...sourceNode.options] : [];
    const nextMaxLen =
      sourceNode?.maxLen && sourceNode.maxLen.length > 0
        ? sourceNode.maxLen[0]
        : undefined;
    const nextDataType = sourceNode?.dataType ?? "";

    editVarId = nextVarId;
    editUnit = nextUnit;
    editMin = nextMin;
    editMax = nextMax;
    editOptionTags = nextOptionTags;
    editMaxLen = nextMaxLen;
    editDataType = nextDataType;
    editTouched = {};
    editError = "";
    editDialogForm.form.set({
      varId: nextVarId ?? "",
      dataType: nextDataType,
      unit: nextUnit,
      min: nextMin,
      max: nextMax,
      options: serializeOptionTags(nextOptionTags),
      maxLen: nextMaxLen,
    });
  }

  function collectFieldErrors<K extends string>(
    issues: { path: PropertyKey[]; message: string }[],
  ): Partial<Record<K, string>> {
    const fieldErrors: Partial<Record<K, string>> = {};
    for (const issue of issues) {
      const key = issue.path[0];
      if (typeof key !== "string") continue;
      if (!fieldErrors[key as K]) {
        fieldErrors[key as K] = issue.message;
      }
    }
    return fieldErrors;
  }

  function touchEditField(key: EditFieldKey): void {
    editTouched = { ...editTouched, [key]: true };
  }

  function touchAllEditFields(): void {
    editTouched = {
      varId: true,
      dataType: true,
      unit: true,
      min: true,
      max: true,
      options: true,
      maxLen: true,
    };
  }

  const editFieldErrors = $derived.by(() => {
    const parsed = editTreeMetaSchema.safeParse({
      varId: editVarId ?? "",
      dataType: editDataType,
      unit: editUnit,
      min: editMin,
      max: editMax,
      options: serializeOptionTags(editOptionTags),
      maxLen: editMaxLen,
    });
    if (parsed.success) return {} as Partial<Record<EditFieldKey, string>>;
    return collectFieldErrors<EditFieldKey>(parsed.error.issues);
  });

  const editFormValid = $derived(Object.keys(editFieldErrors).length === 0);

  function editFieldMessage(key: EditFieldKey): string {
    return editTouched[key] ? (editFieldErrors[key] ?? "") : "";
  }

  function firstValidationError(
    errors: Record<string, unknown> | undefined,
  ): string | null {
    if (!errors) return null;
    for (const value of Object.values(errors)) {
      if (Array.isArray(value) && value.length > 0 && typeof value[0] === "string") {
        return value[0];
      }
    }
    return null;
  }

  function closeDialog(): void {
    open = false;
  }

  async function submitEditDialog(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (!editVarId || !node) return;
    editError = "";
    editSubmitting = true;

    try {
      if (!editFormValid) {
        touchAllEditFields();
        editError = firstValidationError(editFieldErrors) ?? "Invalid form input";
        editSubmitting = false;
        return;
      }

      const editOptionsValue = serializeOptionTags(editOptionTags);
      editDialogForm.form.set({
        varId: editVarId,
        dataType: editDataType,
        unit: editUnit,
        min: editMin,
        max: editMax,
        options: editOptionsValue,
        maxLen: editMaxLen,
      });

      const sfValidation = await editDialogForm.validateForm({
        update: false,
        schema: zod4(editTreeMetaSchema),
      });
      if (!sfValidation.valid) {
        editError = firstValidationError(sfValidation.errors) ?? "Invalid form input";
        editSubmitting = false;
        return;
      }

      const parsed = editTreeMetaSchema.safeParse({
        varId: editVarId,
        dataType: editDataType,
        unit: editUnit,
        min: editMin,
        max: editMax,
        options: editOptionsValue,
        maxLen: editMaxLen,
      });
      if (!parsed.success) {
        editError = parsed.error.issues[0]?.message ?? "Invalid form input";
        editSubmitting = false;
        return;
      }

      const payload = toEditMetaPayload(parsed.data);
      await onEditMeta(payload);
      await onRefreshParent(node.parentId ?? null);
      closeDialog();
    } catch (error) {
      editError =
        error instanceof Error ? error.message : "Failed to update metadata";
    } finally {
      editSubmitting = false;
    }
  }

  $effect(() => {
    if (!dialogEl) {
      return;
    }
    if (open) {
      hydrateFromNode(node);
      if (!dialogEl.open) {
        dialogEl.showModal();
      }
      return;
    }
    if (dialogEl.open) {
      dialogEl.close();
    }
  });
</script>

<dialog
  bind:this={dialogEl}
  class="fixed inset-0 m-auto w-[420px] max-w-[90vw] rounded-md border border-border bg-card p-0 text-foreground shadow-xl backdrop:bg-black/50"
  onclose={() => {
    open = false;
  }}
>
  <form class="flex h-full flex-col p-4" onsubmit={submitEditDialog} novalidate>
    <div class="flex items-center justify-between pb-4">
      <h2 class="text-sm font-semibold">Edit Variable Metadata</h2>
    </div>

    <div class="space-y-3 overflow-y-auto pr-1">
      <div class="space-y-1">
        <span class="text-xs text-muted-foreground">Variable ID</span>
        <div class="rounded border border-border bg-muted/50 px-2 py-1.5 text-[11px] text-muted-foreground">
          {editVarId}
        </div>
      </div>

      <div class="space-y-1">
        <label class="text-xs text-muted-foreground" for="edit-unit">Unit</label>
        <Input
          id="edit-unit"
          type="text"
          class="w-full"
          bind:value={editUnit}
          onblur={() => touchEditField("unit")}
          placeholder="e.g. °C, kPa"
        />
        {#if editFieldMessage("unit")}
          <p class="text-xs text-red-500">{editFieldMessage("unit")}</p>
        {/if}
      </div>

      {#if editDataType === "VAR_DATA_TYPE_INTEGER" || editDataType === "VAR_DATA_TYPE_FLOAT"}
        <div class="grid grid-cols-2 gap-2">
          <div class="space-y-1">
            <label class="text-xs text-muted-foreground" for="edit-min">Min</label>
            <NumberField
              id="edit-min"
              step="any"
              class="w-full"
              bind:value={editMin}
              onblur={() => touchEditField("min")}
              placeholder="e.g. 0"
            />
            {#if editFieldMessage("min")}
              <p class="text-xs text-red-500">{editFieldMessage("min")}</p>
            {/if}
          </div>
          <div class="space-y-1">
            <label class="text-xs text-muted-foreground" for="edit-max">Max</label>
            <NumberField
              id="edit-max"
              step="any"
              class="w-full"
              bind:value={editMax}
              onblur={() => touchEditField("max")}
              placeholder="e.g. 100"
            />
            {#if editFieldMessage("max")}
              <p class="text-xs text-red-500">{editFieldMessage("max")}</p>
            {/if}
          </div>
        </div>
      {:else if editDataType === "VAR_DATA_TYPE_TEXT"}
        <div class="space-y-1">
          <label class="text-xs text-muted-foreground" for="edit-options">Options</label>
          <TagsInput
            id="edit-options"
            class="w-full text-sm"
            bind:value={editOptionTags}
            commitSeparators={[","]}
            onValueChange={() => touchEditField("options")}
            placeholder="Type an option and press comma or Enter"
          />
          {#if editFieldMessage("options")}
            <p class="text-xs text-red-500">{editFieldMessage("options")}</p>
          {/if}
        </div>
        <div class="space-y-1">
          <label class="text-xs text-muted-foreground" for="edit-maxlen">Max length</label>
          <NumberField
            id="edit-maxlen"
            step={1}
            min={0}
            class="w-full"
            bind:value={editMaxLen}
            onblur={() => touchEditField("maxLen")}
            placeholder="e.g. 32"
          />
          {#if editFieldMessage("maxLen")}
            <p class="text-xs text-red-500">{editFieldMessage("maxLen")}</p>
          {/if}
        </div>
      {/if}
    </div>

    {#if editError}
      <p class="pt-2 text-xs text-red-500">{editError}</p>
    {/if}

    <div class="mt-auto flex justify-end gap-2 border-t border-border pt-4">
      <Button
        variant="outline-muted"
        label="Cancel"
        title="Cancel"
        onclick={closeDialog}
        type="button"
      />
      <Button
        type="submit"
        variant="filled-accent"
        label="Save"
        title="Save"
        loading={editSubmitting}
        loadingLabel="Saving…"
        disabled={editSubmitting || !connected || !editFormValid}
      />
    </div>
  </form>
</dialog>
