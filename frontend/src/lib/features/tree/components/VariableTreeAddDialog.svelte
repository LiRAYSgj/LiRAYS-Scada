<script lang="ts">
  import { defaults, superForm } from "sveltekit-superforms/client";
  import { zod4, zod4Client } from "sveltekit-superforms/adapters";
  import { ItemType, VarDataType } from "@lirays/scada-proto";
  import { addTreeItemSchema, toCreateItemPayload } from "$lib/forms/tree-schemas";
  import { Button } from "$lib/components/Button";
  import { Input } from "$lib/components/ui/input";
  import { NumberField } from "$lib/components/ui/number-field";
  import { TagsInput } from "$lib/components/ui/tags-input";
  import * as Select from "$lib/components/ui/select";

  interface Props {
    open?: boolean;
    connected: boolean;
    parentId: string | null;
    onCreateItem: (input: {
      parentId: string | null;
      name: string;
      itemType: ItemType;
      varType: VarDataType | undefined;
      unit?: string;
      min?: number;
      max?: number;
      options?: string[];
      maxLen?: number;
    }) => Promise<void>;
  }

  type AddFieldKey =
    | "name"
    | "kind"
    | "dataType"
    | "unit"
    | "min"
    | "max"
    | "options"
    | "maxLen";

  let {
    open = $bindable(false),
    connected,
    parentId,
    onCreateItem,
  }: Props = $props();

  const addDialogForm = superForm(
    defaults(
      {
        name: "",
        kind: ItemType.ITEM_TYPE_VARIABLE,
        dataType: VarDataType.VAR_DATA_TYPE_TEXT,
        unit: "",
        min: undefined,
        max: undefined,
        options: "",
        maxLen: undefined,
      },
      zod4(addTreeItemSchema),
    ),
    {
      SPA: true,
      validators: zod4Client(addTreeItemSchema),
      validationMethod: "submit-only",
    },
  );

  let dialogEl: HTMLDialogElement | null = null;
  let addName = $state("");
  let addKindValue = $state<string>(String(ItemType.ITEM_TYPE_VARIABLE));
  let addDataTypeValue = $state<string>(String(VarDataType.VAR_DATA_TYPE_TEXT));
  const addKind = $derived(Number(addKindValue) as ItemType);
  const addDataType = $derived(Number(addDataTypeValue) as VarDataType);
  let addUnit = $state("");
  let addMin = $state<number | undefined>(undefined);
  let addMax = $state<number | undefined>(undefined);
  let addOptionTags = $state<string[]>([]);
  let addMaxLen = $state<number | undefined>(undefined);
  let addError = $state("");
  let addSubmitting = $state(false);
  let addTouched = $state<Partial<Record<AddFieldKey, boolean>>>({});

  function serializeOptionTags(tags: string[]): string {
    return tags.join(",");
  }

  function resetForm(): void {
    addName = "";
    addKindValue = String(ItemType.ITEM_TYPE_VARIABLE);
    addDataTypeValue = String(VarDataType.VAR_DATA_TYPE_TEXT);
    addUnit = "";
    addMin = undefined;
    addMax = undefined;
    addOptionTags = [];
    addMaxLen = undefined;
    addTouched = {};
    addError = "";
    addDialogForm.form.set({
      name: "",
      kind: ItemType.ITEM_TYPE_VARIABLE,
      dataType: VarDataType.VAR_DATA_TYPE_TEXT,
      unit: "",
      min: undefined,
      max: undefined,
      options: "",
      maxLen: undefined,
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

  function touchAddField(key: AddFieldKey): void {
    addTouched = { ...addTouched, [key]: true };
  }

  function touchAllAddFields(): void {
    addTouched = {
      name: true,
      kind: true,
      dataType: true,
      unit: true,
      min: true,
      max: true,
      options: true,
      maxLen: true,
    };
  }

  const addFieldErrors = $derived.by(() => {
    const parsed = addTreeItemSchema.safeParse({
      name: addName,
      kind: addKind,
      dataType: addDataType,
      unit: addUnit,
      min: addMin,
      max: addMax,
      options: serializeOptionTags(addOptionTags),
      maxLen: addMaxLen,
    });
    if (parsed.success) return {} as Partial<Record<AddFieldKey, string>>;
    return collectFieldErrors<AddFieldKey>(parsed.error.issues);
  });

  const addFormValid = $derived(Object.keys(addFieldErrors).length === 0);

  function addFieldMessage(key: AddFieldKey): string {
    return addTouched[key] ? (addFieldErrors[key] ?? "") : "";
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

  async function submitAddDialog(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    addError = "";
    addSubmitting = true;

    try {
      if (!addFormValid) {
        touchAllAddFields();
        addError = firstValidationError(addFieldErrors) ?? "Invalid form input";
        addSubmitting = false;
        return;
      }

      const addOptionsValue = serializeOptionTags(addOptionTags);
      addDialogForm.form.set({
        name: addName,
        kind: addKind,
        dataType: addDataType,
        unit: addUnit,
        min: addMin,
        max: addMax,
        options: addOptionsValue,
        maxLen: addMaxLen,
      });

      const sfValidation = await addDialogForm.validateForm({
        update: false,
        schema: zod4(addTreeItemSchema),
      });
      if (!sfValidation.valid) {
        addError = firstValidationError(sfValidation.errors) ?? "Invalid form input";
        addSubmitting = false;
        return;
      }

      const parsed = addTreeItemSchema.safeParse({
        name: addName,
        kind: addKind,
        dataType: addDataType,
        unit: addUnit,
        min: addMin,
        max: addMax,
        options: addOptionsValue,
        maxLen: addMaxLen,
      });
      if (!parsed.success) {
        addError = parsed.error.issues[0]?.message ?? "Invalid form input";
        addSubmitting = false;
        return;
      }

      const payload = toCreateItemPayload(parsed.data);
      await onCreateItem({
        parentId,
        name: payload.name,
        itemType: payload.itemType,
        varType: payload.varType,
        unit: payload.unit,
        min: payload.min,
        max: payload.max,
        options: payload.options,
        maxLen: payload.maxLen,
      });
      closeDialog();
    } catch (error) {
      addError =
        error instanceof Error ? error.message : "Failed to create item";
    } finally {
      addSubmitting = false;
    }
  }

  $effect(() => {
    if (!dialogEl) {
      return;
    }
    if (open) {
      resetForm();
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
  <form class="flex h-full flex-col p-4" onsubmit={submitAddDialog} novalidate>
    <div class="flex items-center justify-between pb-4">
      <h2 class="text-sm font-semibold">Add Variable/Folder</h2>
    </div>

    <div class="space-y-4 overflow-y-auto pr-1">
      <div class="space-y-1">
        <label class="text-xs text-muted-foreground" for="add-name">Name</label>
        <Input
          id="add-name"
          type="text"
          class="w-full"
          bind:value={addName}
          oninput={(event) => {
            const target = event.currentTarget as HTMLInputElement;
            if (target.value.includes("/")) {
              const cleaned = target.value.replaceAll("/", "");
              target.value = cleaned;
              addName = cleaned;
            }
          }}
          onblur={() => touchAddField("name")}
          placeholder="Enter node name"
        />
        {#if addFieldMessage("name")}
          <p class="text-xs text-red-500">{addFieldMessage("name")}</p>
        {/if}
      </div>

      <div class="space-y-1">
        <label class="text-xs text-muted-foreground" for="add-kind">Node Type</label>
        <Select.Root
          type="single"
          bind:value={addKindValue}
          onValueChange={() => touchAddField("kind")}
        >
          <Select.Trigger id="add-kind" class="w-full">
            {addKindValue === String(ItemType.ITEM_TYPE_FOLDER) ? "Folder" : "Variable"}
          </Select.Trigger>
          <Select.Content
            portalProps={{ disabled: true }}
            class="z-[80] border border-border bg-card shadow-lg"
          >
            <Select.Group>
              <Select.Item value={String(ItemType.ITEM_TYPE_FOLDER)} label="Folder" />
              <Select.Item value={String(ItemType.ITEM_TYPE_VARIABLE)} label="Variable" />
            </Select.Group>
          </Select.Content>
        </Select.Root>
        {#if addFieldMessage("kind")}
          <p class="text-xs text-red-500">{addFieldMessage("kind")}</p>
        {/if}
      </div>

      <div class={`space-y-1 ${addKind !== ItemType.ITEM_TYPE_VARIABLE ? "invisible" : ""}`}>
        <label class="text-xs text-muted-foreground" for="add-dataType">Data Type</label>
        <Select.Root
          type="single"
          bind:value={addDataTypeValue}
          onValueChange={() => touchAddField("dataType")}
          disabled={addKind !== ItemType.ITEM_TYPE_VARIABLE}
        >
          <Select.Trigger id="add-dataType" class="w-full">
            {#if addDataTypeValue === String(VarDataType.VAR_DATA_TYPE_INTEGER)}
              Integer
            {:else if addDataTypeValue === String(VarDataType.VAR_DATA_TYPE_FLOAT)}
              Float
            {:else if addDataTypeValue === String(VarDataType.VAR_DATA_TYPE_TEXT)}
              Text
            {:else}
              Boolean
            {/if}
          </Select.Trigger>
          <Select.Content
            portalProps={{ disabled: true }}
            class="z-[80] border border-border bg-card shadow-lg"
          >
            <Select.Group>
              <Select.Item value={String(VarDataType.VAR_DATA_TYPE_INTEGER)} label="Integer" />
              <Select.Item value={String(VarDataType.VAR_DATA_TYPE_FLOAT)} label="Float" />
              <Select.Item value={String(VarDataType.VAR_DATA_TYPE_TEXT)} label="Text" />
              <Select.Item value={String(VarDataType.VAR_DATA_TYPE_BOOLEAN)} label="Boolean" />
            </Select.Group>
          </Select.Content>
        </Select.Root>
        {#if addFieldMessage("dataType")}
          <p class="text-xs text-red-500">{addFieldMessage("dataType")}</p>
        {/if}
      </div>

      {#if addKind === ItemType.ITEM_TYPE_VARIABLE}
        <div class="space-y-3">
          <div class="space-y-1">
            <label class="text-xs text-muted-foreground" for="add-unit">Unit</label>
            <Input
              id="add-unit"
              type="text"
              class="w-full"
              bind:value={addUnit}
              onblur={() => touchAddField("unit")}
              placeholder="e.g. °C, kPa"
            />
            {#if addFieldMessage("unit")}
              <p class="text-xs text-red-500">{addFieldMessage("unit")}</p>
            {/if}
          </div>

          {#if addDataType === VarDataType.VAR_DATA_TYPE_INTEGER || addDataType === VarDataType.VAR_DATA_TYPE_FLOAT}
            <div class="grid grid-cols-2 gap-2">
              <div class="space-y-1">
                <label class="text-xs text-muted-foreground" for="add-min">Min</label>
                <NumberField
                  id="add-min"
                  step="any"
                  class="w-full"
                  bind:value={addMin}
                  onblur={() => touchAddField("min")}
                  placeholder="e.g. 0"
                />
                {#if addFieldMessage("min")}
                  <p class="text-xs text-red-500">{addFieldMessage("min")}</p>
                {/if}
              </div>
              <div class="space-y-1">
                <label class="text-xs text-muted-foreground" for="add-max">Max</label>
                <NumberField
                  id="add-max"
                  step="any"
                  class="w-full"
                  bind:value={addMax}
                  onblur={() => touchAddField("max")}
                  placeholder="e.g. 100"
                />
                {#if addFieldMessage("max")}
                  <p class="text-xs text-red-500">{addFieldMessage("max")}</p>
                {/if}
              </div>
            </div>
          {:else if addDataType === VarDataType.VAR_DATA_TYPE_TEXT}
            <div class="space-y-1">
              <label class="text-xs text-muted-foreground" for="add-options">Allowed options</label>
              <TagsInput
                id="add-options"
                class="w-full text-sm"
                bind:value={addOptionTags}
                commitSeparators={[","]}
                onValueChange={() => touchAddField("options")}
                placeholder="Type an option and press comma or Enter"
              />
              {#if addFieldMessage("options")}
                <p class="text-xs text-red-500">{addFieldMessage("options")}</p>
              {/if}
            </div>
            <div class="space-y-1">
              <label class="text-xs text-muted-foreground" for="add-maxlen">Max length</label>
              <NumberField
                id="add-maxlen"
                step={1}
                min={0}
                class="w-full"
                bind:value={addMaxLen}
                onblur={() => touchAddField("maxLen")}
                placeholder="e.g. 32"
              />
              {#if addFieldMessage("maxLen")}
                <p class="text-xs text-red-500">{addFieldMessage("maxLen")}</p>
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    </div>

    {#if addError}
      <p class="pt-2 text-xs text-red-500">{addError}</p>
    {/if}

    <div class="mt-auto flex justify-end gap-2 border-t border-border pt-4">
      <Button
        type="button"
        variant="outline-muted"
        label="Cancel"
        title="Cancel"
        onclick={closeDialog}
      />
      <Button
        type="submit"
        variant="filled-accent"
        label="Save"
        title="Save"
        loading={addSubmitting}
        loadingLabel="Saving…"
        disabled={addSubmitting || !connected || !addFormValid}
      />
    </div>
  </form>
</dialog>
