<script lang="ts">
  import { get } from "svelte/store";
  import { onMount, untrack } from "svelte";
  import { Button } from "$lib/components/Button";
  import { Input } from "$lib/components/ui/input";
  import { NumberField } from "$lib/components/ui/number-field";
  import { TagsInput } from "$lib/components/ui/tags-input";
  import * as Select from "$lib/components/ui/select";
  import { defaults, superForm } from "sveltekit-superforms/client";
  import { zod4, zod4Client } from "sveltekit-superforms/adapters";
  import { Circle, LoaderCircle } from "lucide-svelte";
  import TreeRow from "./TreeRow.svelte";
  import TagMetadataTooltip from "./TagMetadataTooltip.svelte";
  import { fetchTreeChildren } from "../server-adapter";
  import { createTreeStore } from "../tree-store";
  import {
    getLoadedDescendantIds,
    hasPartialSelectionInSubtree,
  } from "../tree-selection";
  import type { TreeNode } from "../types";
  import {
    type TagScalarValue,
    WebSocketConnectionStatus,
  } from "$lib/core/ws/types";
  import { tagStreamClient } from "$lib/core/ws/tag-stream-client";
  import { ItemType, VarDataType } from "$lib/proto/namespace/enums";
  import {
    addTreeItemSchema,
    editTreeMetaSchema,
    toCreateItemPayload,
    toEditMetaPayload,
  } from "$lib/forms/tree-schemas";

  export interface SelectionChangePayload {
    add: string[];
    remove: string[];
  }

  interface Props {
    onNodeContextMenu: (event: MouseEvent, node: TreeNode) => void;
    onNodeDragStart: (event: DragEvent, node: TreeNode) => void;
    onNodeDragEnd: (event: DragEvent) => void;
    websocketStatus?: WebSocketConnectionStatus;
    realtimeEnabled?: boolean;
    liveTagValues?: Record<string, TagScalarValue>;
    /** Called when root node id(s) are known (e.g. for namespace builder parentId at root). */
    onRootId?: (id: string | null) => void;
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
    onEditMeta: (input: {
      varId: string;
      unit?: string;
      min?: number;
      max?: number;
      options?: string[];
      maxLen?: number;
    }) => Promise<void>;
    /** When true, show checkboxes and use multi-selection instead of single select/drag/context. */
    multiSelectMode?: boolean;
    /** Set of selected node ids (used when multiSelectMode is true). */
    selection?: Set<string>;
    /** When true, checking a parent adds all loaded descendants; unchecking removes them. */
    propagateDown?: boolean;
    /** When true, checking a node adds parent (and ancestors) if all siblings are selected. */
    propagateUp?: boolean;
    /** Called when user toggles selection (checkbox). */
    onSelectionChange?: (payload: SelectionChangePayload) => void;
    /** Called when tree state changes so parent can compute minimal set for delete (nodes, rootIds). */
    onTreeStateSnapshot?: (
      nodes: Record<string, TreeNode>,
      rootIds: string[],
    ) => void;
  }

  let {
    onNodeContextMenu,
    onNodeDragStart,
    onNodeDragEnd,
    websocketStatus = WebSocketConnectionStatus.DISCONNECTED,
    realtimeEnabled = false,
    liveTagValues = {},
    onRootId,
    onCreateItem,
    onEditMeta,
    multiSelectMode = false,
    selection = new Set<string>(),
    propagateDown = true,
    propagateUp = true,
    onSelectionChange,
    onTreeStateSnapshot,
  }: Props = $props();

  const tree = createTreeStore({
    fetchChildren: fetchTreeChildren,
  });
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

  const treeState = tree.state;
  const visibleRows = tree.visibleRows;
  const skeletonRows = Array.from(
    { length: 14 },
    (_, index) => `skeleton-${index}`,
  );
  const ROW_HEIGHT = 32;
  const OVERSCAN = 10;
  const TAG_TOOLTIP_WIDTH = 240;
  const TAG_TOOLTIP_OFFSET_X = 12;
  const TAG_TOOLTIP_MARGIN = 8;
  const TAG_TOOLTIP_MAX_HEIGHT = 220;

  interface TagTooltipState {
    node: TreeNode;
    x: number;
    y: number;
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
  type EditFieldKey =
    | "varId"
    | "dataType"
    | "unit"
    | "min"
    | "max"
    | "options"
    | "maxLen";

  let treeViewportEl: HTMLDivElement | null = null;
  let addDialog: HTMLDialogElement | null = null;
  let editDialog: HTMLDialogElement | null = null;
  let tagTooltip = $state<TagTooltipState | null>(null);
  let scrollTop = $state(0);
  let viewportHeight = $state(0);
  let addName = $state("");
  let addKind = $state<ItemType>(ItemType.ITEM_TYPE_VARIABLE);
  let addDataType = $state<VarDataType>(VarDataType.VAR_DATA_TYPE_TEXT);
  let addKindValue = $state<string>(String(ItemType.ITEM_TYPE_VARIABLE));
  let addDataTypeValue = $state<string>(String(VarDataType.VAR_DATA_TYPE_TEXT));
  let addUnit = $state("");
  let addMin = $state<number | undefined>(undefined);
  let addMax = $state<number | undefined>(undefined);
  let addOptions = $state("");
  let addOptionTags = $state<string[]>([]);
  let addMaxLen = $state<number | undefined>(undefined);
  let addError = $state("");
  let addSubmitting = $state(false);
  let addParentId = $state<string | null | undefined>(undefined);
  let addTouched = $state<Partial<Record<AddFieldKey, boolean>>>({});
  let addCancelling = $state(false);
  let editVarId = $state<string | null>(null);
  let editUnit = $state("");
  let editMin = $state<number | undefined>(undefined);
  let editMax = $state<number | undefined>(undefined);
  let editOptions = $state("");
  let editOptionTags = $state<string[]>([]);
  let editMaxLen = $state<number | undefined>(undefined);
  let editDataType = $state<string>("");
  let editError = $state("");
  let editSubmitting = $state(false);
  let editTouched = $state<Partial<Record<EditFieldKey, boolean>>>({});
  let editCancelling = $state(false);
  const isConnected = $derived(
    websocketStatus === WebSocketConnectionStatus.CONNECTED,
  );

  $effect(() => {
    addKind = Number(addKindValue) as ItemType;
  });

  $effect(() => {
    addDataType = Number(addDataTypeValue) as VarDataType;
  });

  function serializeOptionTags(tags: string[]): string {
    return tags.join(",");
  }

  function addSnapshot() {
    return {
      name: addName,
      kind: addKind,
      dataType: addDataType,
      unit: addUnit,
      min: addMin,
      max: addMax,
      options: serializeOptionTags(addOptionTags),
      maxLen: addMaxLen,
    };
  }

  function editSnapshot() {
    return {
      varId: editVarId ?? "",
      dataType: editDataType,
      unit: editUnit,
      min: editMin,
      max: editMax,
      options: serializeOptionTags(editOptionTags),
      maxLen: editMaxLen,
    };
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

  const addFieldErrors = $derived.by(() => {
    const parsed = addTreeItemSchema.safeParse(addSnapshot());
    if (parsed.success) return {} as Partial<Record<AddFieldKey, string>>;
    return collectFieldErrors<AddFieldKey>(parsed.error.issues);
  });

  const editFieldErrors = $derived.by(() => {
    const parsed = editTreeMetaSchema.safeParse(editSnapshot());
    if (parsed.success) return {} as Partial<Record<EditFieldKey, string>>;
    return collectFieldErrors<EditFieldKey>(parsed.error.issues);
  });

  const addFormValid = $derived(Object.keys(addFieldErrors).length === 0);
  const editFormValid = $derived(Object.keys(editFieldErrors).length === 0);

  function touchAddField(key: AddFieldKey): void {
    if (addCancelling) return;
    addTouched = { ...addTouched, [key]: true };
  }

  function touchEditField(key: EditFieldKey): void {
    if (editCancelling) return;
    editTouched = { ...editTouched, [key]: true };
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

  function addFieldMessage(key: AddFieldKey): string {
    return addTouched[key] ? (addFieldErrors[key] ?? "") : "";
  }

  function editFieldMessage(key: EditFieldKey): string {
    return editTouched[key] ? (editFieldErrors[key] ?? "") : "";
  }

  $effect(() => {
    addOptions = serializeOptionTags(addOptionTags);
  });

  $effect(() => {
    editOptions = serializeOptionTags(editOptionTags);
  });

  $effect(() => {
    if (!onRootId) return;
    const state = $treeState;
    if (state.rootIds.length > 0) {
      onRootId(state.rootIds[0]);
    } else if (state.hasInitialized) {
      onRootId(null);
    }
  });

  $effect(() => {
    if (!onTreeStateSnapshot) return;
    const state = $treeState;
    onTreeStateSnapshot(state.nodes, state.rootIds);
  });

  /** When in multi-select with propagateDown: add newly loaded descendants of selected nodes (e.g. after expanding). Only depends on tree state so user unchecking a child does not re-add it. */
  $effect(() => {
    if (
      !multiSelectMode ||
      !propagateDown ||
      !onSelectionChange
    ) return;
    const state = $treeState;
    const nodes = state.nodes;
    const sel = untrack(() => selection);
    const toAddSet = new Set<string>();
    for (const selectedId of sel) {
      for (const id of getLoadedDescendantIds(selectedId, nodes)) {
        if (!sel.has(id)) toAddSet.add(id);
      }
    }
    if (toAddSet.size > 0) {
      onSelectionChange({ add: [...toAddSet], remove: [] });
    }
  });

  function handleSelectionCheckClick(nodeId: string): void {
    if (!onSelectionChange) return;
    const state = get(treeState);
    const nodes = state.nodes;
    const currentlyChecked = selection.has(nodeId);
    const newChecked = !currentlyChecked;

    const add: string[] = [];
    const remove: string[] = [];

    if (newChecked) {
      add.push(nodeId);
      if (propagateDown) {
        add.push(...getLoadedDescendantIds(nodeId, nodes));
      }
      if (propagateUp) {
        let parentId: string | null = nodes[nodeId]?.parentId ?? null;
        while (parentId) {
          const parent = nodes[parentId];
          if (!parent?.childIds) break;
          const allSiblingsSelected = parent.childIds.every((id) =>
            id === nodeId || selection.has(id) || add.includes(id),
          );
          if (!allSiblingsSelected) break;
          add.push(parentId);
          parentId = parent.parentId ?? null;
        }
      }
    } else {
      remove.push(nodeId);
      if (propagateDown) {
        remove.push(...getLoadedDescendantIds(nodeId, nodes));
      }
    }

    onSelectionChange({ add, remove });
  }

  const totalRows = $derived($visibleRows.length);
  const startIndex = $derived(
    Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN),
  );
  const visibleCount = $derived(
    Math.max(1, Math.ceil(viewportHeight / ROW_HEIGHT) + OVERSCAN * 2),
  );
  const endIndex = $derived(Math.min(totalRows, startIndex + visibleCount));
  const windowRows = $derived($visibleRows.slice(startIndex, endIndex));
  const topPadding = $derived(startIndex * ROW_HEIGHT);
  const bottomPadding = $derived((totalRows - endIndex) * ROW_HEIGHT);

  onMount(() => {
    void tree.initialize();

    const unsubTree = tagStreamClient.treeChanges.subscribe((ev) => {
      if (ev?.folderChangedEvent?.length) {
        void tree.applyRemoteTreeChanged(ev);
      }
    });

    if (!treeViewportEl) {
      return () => {
        unsubTree();
      };
    }

    viewportHeight = treeViewportEl.clientHeight;
    const observer = new ResizeObserver(() => {
      if (!treeViewportEl) {
        return;
      }
      viewportHeight = treeViewportEl.clientHeight;
    });
    observer.observe(treeViewportEl);

    const refreshHandler = (event: Event) => {
      const custom = event as CustomEvent<{ parentId?: string | null }>;
      void tree.refreshNode(custom.detail?.parentId ?? null);
    };
    window.addEventListener("tree:refresh", refreshHandler as EventListener);
    const openAddDialogHandler = (event: Event) => {
      const custom = event as CustomEvent<{ parentId?: string | null }>;
      addParentId = custom.detail?.parentId;
      openAddDialog();
    };
    window.addEventListener(
      "tree:open-add-dialog",
      openAddDialogHandler as EventListener,
    );
    const openEditDialogHandler = (event: Event) => {
      const custom = event as CustomEvent<{ node: TreeNode }>;
      if (!custom.detail?.node) return;
      openEditDialog(custom.detail.node);
    };
    window.addEventListener(
      "tree:open-edit-dialog",
      openEditDialogHandler as EventListener,
    );

    return () => {
      unsubTree();
      observer.disconnect();
      window.removeEventListener(
        "tree:refresh",
        refreshHandler as EventListener,
      );
      window.removeEventListener(
        "tree:open-add-dialog",
        openAddDialogHandler as EventListener,
      );
      window.removeEventListener(
        "tree:open-edit-dialog",
        openEditDialogHandler as EventListener,
      );
    };
  });

  function ensureIndexVisible(index: number): void {
    if (!treeViewportEl) {
      return;
    }

    const rowTop = index * ROW_HEIGHT;
    const rowBottom = rowTop + ROW_HEIGHT;
    const viewportTop = treeViewportEl.scrollTop;
    const viewportBottom = viewportTop + treeViewportEl.clientHeight;

    if (rowTop < viewportTop) {
      treeViewportEl.scrollTop = rowTop;
    } else if (rowBottom > viewportBottom) {
      treeViewportEl.scrollTop = rowBottom - treeViewportEl.clientHeight;
    }
  }

  function selectNode(nodeId: string): void {
    tree.selectNode(nodeId);
  }

  function toggleNode(nodeId: string): void {
    tree.toggleExpanded(nodeId);
  }

  function handleTreeKeyDown(event: KeyboardEvent): void {
    const rows = get(visibleRows);
    const currentState = get(treeState);
    if (rows.length === 0) {
      return;
    }

    const selectedId = currentState.selectedId ?? rows[0].id;
    const selectedIndex = Math.max(
      0,
      rows.findIndex((row) => row.id === selectedId),
    );
    const selectedRow = rows[selectedIndex];

    if (event.key === "ArrowDown") {
      event.preventDefault();
      const nextIndex = Math.min(rows.length - 1, selectedIndex + 1);
      const next = rows[nextIndex];
      tree.selectNode(next.id);
      ensureIndexVisible(nextIndex);
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      const prevIndex = Math.max(0, selectedIndex - 1);
      const prev = rows[prevIndex];
      tree.selectNode(prev.id);
      ensureIndexVisible(prevIndex);
      return;
    }

    if (event.key === "ArrowRight") {
      event.preventDefault();
      if (selectedRow.node.hasChildren && !selectedRow.isExpanded) {
        tree.toggleExpanded(selectedRow.id);
        return;
      }

      const childId = selectedRow.node.childIds?.[0];
      if (childId) {
        tree.selectNode(childId);
      }
      return;
    }

    if (event.key === "ArrowLeft") {
      event.preventDefault();
      if (selectedRow.isExpanded) {
        tree.collapseNode(selectedRow.id);
        return;
      }

      if (selectedRow.node.parentId) {
        tree.selectNode(selectedRow.node.parentId);
      }
      return;
    }

    if (event.key === "Home") {
      event.preventDefault();
      tree.selectNode(rows[0].id);
      ensureIndexVisible(0);
      return;
    }

    if (event.key === "End") {
      event.preventDefault();
      tree.selectNode(rows[rows.length - 1].id);
      ensureIndexVisible(rows.length - 1);
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      tree.selectNode(selectedRow.id);
    }
  }

  function openAddDialog(): void {
    if (!isConnected || !addDialog) {
      return;
    }
    addName = "";
    addKind = ItemType.ITEM_TYPE_VARIABLE;
    addDataType = VarDataType.VAR_DATA_TYPE_TEXT;
    addKindValue = String(ItemType.ITEM_TYPE_VARIABLE);
    addDataTypeValue = String(VarDataType.VAR_DATA_TYPE_TEXT);
    addUnit = "";
    addMin = undefined;
    addMax = undefined;
    addOptions = "";
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
    addDialog.showModal();
  }

  function openEditDialog(node: TreeNode): void {
    if (!isConnected || !editDialog || node.kind !== "tag") {
      return;
    }
    editVarId = node.id;
    editUnit = node.unit ?? "";
    editMin = node.min;
    editMax = node.max;
    editOptionTags = node.options ? [...node.options] : [];
    editOptions = serializeOptionTags(editOptionTags);
    editMaxLen = node.maxLen && node.maxLen.length > 0 ? node.maxLen[0] : undefined;
    editDataType = node.dataType ?? "";
    editTouched = {};
    editError = "";
    editDialogForm.form.set({
      varId: node.id,
      dataType: node.dataType ?? "",
      unit: node.unit ?? "",
      min: node.min,
      max: node.max,
      options: serializeOptionTags(editOptionTags),
      maxLen: node.maxLen && node.maxLen.length > 0 ? node.maxLen[0] : undefined,
    });
    editDialog.showModal();
  }

  function closeAddDialog(): void {
    addCancelling = false;
    addTouched = {};
    addError = "";
    addParentId = undefined;
    if (addDialog?.open) {
      addDialog.close();
    }
  }

  function closeEditDialog(): void {
    editCancelling = false;
    editTouched = {};
    editError = "";
    if (editDialog?.open) {
      editDialog.close();
    }
    editVarId = null;
  }

  function resolveParentIdForCreate(): string | null {
    if (addParentId !== undefined) {
      return addParentId;
    }

    const snapshot = get(treeState);
    const selected = snapshot.selectedId
      ? snapshot.nodes[snapshot.selectedId]
      : null;
    if (selected?.kind === "folder") {
      return selected.id;
    }
    if (selected?.kind === "tag") {
      return selected.parentId;
    }
    return snapshot.rootIds[0] ?? null;
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

  async function submitAddDialog(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    addError = "";

    const parentId = resolveParentIdForCreate();
    if (parentId === null && addParentId !== null) {
      addError = "Cannot resolve parent folder";
      return;
    }

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
        parentId: parentId ?? null,
        name: payload.name,
        itemType: payload.itemType,
        varType: payload.varType,
        unit: payload.unit,
        min: payload.min,
        max: payload.max,
        options: payload.options,
        maxLen: payload.maxLen,
      });
      closeAddDialog();
    } catch (error) {
      addError =
        error instanceof Error ? error.message : "Failed to create item";
    } finally {
      addSubmitting = false;
    }
  }

  async function submitEditDialog(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (!editVarId) return;
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
      const node = get(treeState).nodes[editVarId];
      await tree.refreshNode(node?.parentId ?? null);
      closeEditDialog();
    } catch (error) {
      editError =
        error instanceof Error ? error.message : "Failed to update metadata";
    } finally {
      editSubmitting = false;
    }
  }

  function clamp(value: number, min: number, max: number): number {
    return Math.min(Math.max(value, min), max);
  }

  function hideTagTooltip(): void {
    tagTooltip = null;
  }

  function handleTagTooltipChange(
    payload: { node: TreeNode; anchorRect: DOMRect } | null,
  ): void {
    if (!payload) {
      hideTagTooltip();
      return;
    }

    const maxX = Math.max(
      TAG_TOOLTIP_MARGIN,
      window.innerWidth - TAG_TOOLTIP_WIDTH - TAG_TOOLTIP_MARGIN,
    );
    const maxY = Math.max(
      TAG_TOOLTIP_MARGIN,
      window.innerHeight - TAG_TOOLTIP_MAX_HEIGHT - TAG_TOOLTIP_MARGIN,
    );
    tagTooltip = {
      node: payload.node,
      x: clamp(
        payload.anchorRect.right + TAG_TOOLTIP_OFFSET_X,
        TAG_TOOLTIP_MARGIN,
        maxX,
      ),
      y: clamp(payload.anchorRect.top, TAG_TOOLTIP_MARGIN, maxY),
    };
  }
</script>

<section
  class="flex h-full flex-col rounded-md border border-black/10 bg-(--bg-panel) dark:border-white/10"
  style="background-color: var(--bg-panel);"
>
  <header
    class="grid h-9 grid-cols-[1fr_90px_90px_80px] items-center border-b border-black/10 px-2 text-[11px] tracking-wider text-(--text-muted) uppercase dark:border-white/10"
  >
    <span> </span>
    <span>Type</span>
    <span>Value</span>
    <span>Unit</span>
  </header>

  <div
    class="flex-1 overflow-auto"
    bind:this={treeViewportEl}
    onscroll={() => {
      if (!treeViewportEl) {
        return;
      }
      hideTagTooltip();
      scrollTop = treeViewportEl.scrollTop;
    }}
  >
    {#if !$treeState.hasInitialized || $treeState.rootLoading}
      <div class="space-y-2 p-2">
        {#each skeletonRows as rowKey (rowKey)}
          <div
            class="grid h-8 grid-cols-[1fr_90px_90px_80px] items-center gap-2 rounded px-2"
          >
            <div class="h-3.5 animate-pulse rounded bg-(--bg-muted)"></div>
            <div class="h-3.5 animate-pulse rounded bg-(--bg-muted)"></div>
            <div class="h-3.5 animate-pulse rounded bg-(--bg-muted)"></div>
            <div class="h-3.5 animate-pulse rounded bg-(--bg-muted)"></div>
          </div>
        {/each}
      </div>
    {:else if $visibleRows.length === 0}
      <div class="p-3 text-xs text-(--text-muted)">No nodes available</div>
    {:else}
      <div
        role="tree"
        tabindex="0"
        class="outline-none"
        onkeydown={handleTreeKeyDown}
      >
        <div style={`height: ${topPadding}px`} aria-hidden="true"></div>
        {#each windowRows as row (row.id)}
          <TreeRow
            {row}
            isSelected={$treeState.selectedId === row.id}
            onSelect={() => selectNode(row.id)}
            onToggle={() => toggleNode(row.id)}
            onContextMenu={onNodeContextMenu}
            onDragStart={onNodeDragStart}
            onDragEnd={onNodeDragEnd}
            liveValue={realtimeEnabled && row.node.kind === "tag"
              ? liveTagValues[row.node.id]
              : undefined}
            {multiSelectMode}
            isChecked={multiSelectMode && selection.has(row.id)}
            isIndeterminate={multiSelectMode && hasPartialSelectionInSubtree(row.id, $treeState.nodes, selection)}
            onCheckClick={
              multiSelectMode ? () => handleSelectionCheckClick(row.id) : undefined
            }
            onTagTooltipChange={handleTagTooltipChange}
          />
        {/each}
        <div style={`height: ${bottomPadding}px`} aria-hidden="true"></div>
      </div>
    {/if}
  </div>

  <footer
    class="border-t border-black/10 px-2 py-1 text-[11px] text-(--text-muted) dark:border-white/10"
  >
    <div class="flex items-center justify-start">
      {#if websocketStatus === WebSocketConnectionStatus.CONNECTING || websocketStatus === WebSocketConnectionStatus.RECONNECTING}
        <LoaderCircle
          class="h-3.5 w-3.5 animate-spin text-amber-500"
          aria-label="WebSocket connecting"
        />
      {:else if websocketStatus === WebSocketConnectionStatus.CONNECTED}
        <Circle
          class="h-3.5 w-3.5 fill-emerald-500 text-emerald-500"
          aria-label="WebSocket connected"
        />
      {:else}
        <Circle
          class="h-3.5 w-3.5 fill-red-500 text-red-500"
          aria-label="WebSocket disconnected"
        />
      {/if}
    </div>
  </footer>
</section>

{#if tagTooltip}
  <TagMetadataTooltip
    x={tagTooltip.x}
    y={tagTooltip.y}
    node={tagTooltip.node}
    width={TAG_TOOLTIP_WIDTH}
  />
{/if}

<dialog
  bind:this={addDialog}
  class="fixed inset-0 m-auto w-[420px] max-w-[90vw] rounded-md border border-black/10 bg-(--bg-panel) p-0 text-(--text-primary) shadow-xl backdrop:bg-black/50 dark:border-white/10"
>
  <form class="flex h-full flex-col p-4" onsubmit={submitAddDialog} novalidate>
    <div class="flex items-center justify-between pb-4">
      <h2 class="text-sm font-semibold">Add Variable/Folder</h2>
    </div>

    <div class="space-y-4 overflow-y-auto pr-1">
      <div class="space-y-1">
        <label class="text-xs text-(--text-muted)" for="add-name">Name</label>
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
        <label class="text-xs text-(--text-muted)" for="add-kind"
          >Node Type</label
        >
        <Select.Root
          type="single"
          bind:value={addKindValue}
          onValueChange={() => touchAddField("kind")}
        >
          <Select.Trigger class="w-full">
            {addKindValue === String(ItemType.ITEM_TYPE_FOLDER) ? "Folder" : "Variable"}
          </Select.Trigger>
          <Select.Content
            portalProps={{ disabled: true }}
            class="z-[80] border border-black/15 bg-(--bg-panel) shadow-lg dark:border-white/10"
            style="background-color: var(--bg-panel);"
          >
            <Select.Group>
              <Select.Item
                value={String(ItemType.ITEM_TYPE_FOLDER)}
                label="Folder"
              />
              <Select.Item
                value={String(ItemType.ITEM_TYPE_VARIABLE)}
                label="Variable"
              />
            </Select.Group>
          </Select.Content>
        </Select.Root>
        {#if addFieldMessage("kind")}
          <p class="text-xs text-red-500">{addFieldMessage("kind")}</p>
        {/if}
      </div>

      <div
        class={`space-y-1 ${addKind !== ItemType.ITEM_TYPE_VARIABLE ? "invisible" : ""}`}
      >
        <label class="text-xs text-(--text-muted)" for="add-dataType"
          >Data Type</label
        >
        <Select.Root
          type="single"
          bind:value={addDataTypeValue}
          onValueChange={() => touchAddField("dataType")}
          disabled={addKind !== ItemType.ITEM_TYPE_VARIABLE}
        >
          <Select.Trigger class="w-full">
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
            class="z-[80] border border-black/15 bg-(--bg-panel) shadow-lg dark:border-white/10"
            style="background-color: var(--bg-panel);"
          >
            <Select.Group>
              <Select.Item
                value={String(VarDataType.VAR_DATA_TYPE_INTEGER)}
                label="Integer"
              />
              <Select.Item
                value={String(VarDataType.VAR_DATA_TYPE_FLOAT)}
                label="Float"
              />
              <Select.Item
                value={String(VarDataType.VAR_DATA_TYPE_TEXT)}
                label="Text"
              />
              <Select.Item
                value={String(VarDataType.VAR_DATA_TYPE_BOOLEAN)}
                label="Boolean"
              />
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
            <label class="text-xs text-(--text-muted)" for="add-unit"
              >Unit</label
            >
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
                <label class="text-xs text-(--text-muted)" for="add-min"
                  >Min</label
                >
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
                <label class="text-xs text-(--text-muted)" for="add-max"
                  >Max</label
                >
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
              <label class="text-xs text-(--text-muted)" for="add-options"
                >Allowed options</label
              >
              <TagsInput
                id="add-options"
                class="w-full rounded border border-black/15 bg-(--bg-muted) text-sm dark:border-white/10"
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
              <label class="text-xs text-(--text-muted)" for="add-maxlen"
                >Max length</label
              >
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

    <div
      class="mt-auto flex justify-end gap-2 border-t border-black/10 pt-4 dark:border-white/10"
    >
      <Button
        type="button"
        variant="outline-muted"
        label="Cancel"
        title="Cancel"
        onpointerdown={() => {
          addCancelling = true;
        }}
        onclick={closeAddDialog}
      />
      <Button
        type="submit"
        variant="filled-accent"
        label="Save"
        title="Save"
        loading={addSubmitting}
        loadingLabel="Saving…"
        disabled={addSubmitting || !isConnected || !addFormValid}
      />
    </div>
  </form>
</dialog>

<dialog
  bind:this={editDialog}
  class="fixed inset-0 m-auto w-[420px] max-w-[90vw] rounded-md border border-black/10 bg-(--bg-panel) p-0 text-(--text-primary) shadow-xl backdrop:bg-black/50 dark:border-white/10"
>
  <form class="flex h-full flex-col p-4" onsubmit={submitEditDialog} novalidate>
    <div class="flex items-center justify-between pb-4">
      <h2 class="text-sm font-semibold">Edit Variable Metadata</h2>
    </div>

    <div class="space-y-3 overflow-y-auto pr-1">
      <div class="space-y-1">
        <span class="text-xs text-(--text-muted)">Variable ID</span>
        <div
          class="rounded border border-black/15 bg-(--bg-muted) px-2 py-1.5 text-[11px] text-(--text-secondary) dark:border-white/10"
        >
          {editVarId}
        </div>
      </div>

      <div class="space-y-1">
        <label class="text-xs text-(--text-muted)" for="edit-unit">Unit</label>
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
            <label class="text-xs text-(--text-muted)" for="edit-min">Min</label>
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
            <label class="text-xs text-(--text-muted)" for="edit-max">Max</label>
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
          <label class="text-xs text-(--text-muted)" for="edit-options"
            >Options</label
          >
          <TagsInput
            id="edit-options"
            class="w-full rounded border border-black/15 bg-(--bg-muted) text-sm dark:border-white/10"
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
          <label class="text-xs text-(--text-muted)" for="edit-maxlen"
            >Max length</label
          >
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

    <div
      class="mt-auto flex justify-end gap-2 border-t border-black/10 pt-4 dark:border-white/10"
    >
      <Button
        variant="outline-muted"
        label="Cancel"
        title="Cancel"
        onpointerdown={() => {
          editCancelling = true;
        }}
        onclick={closeEditDialog}
        type="button"
      />
      <Button
        type="submit"
        variant="filled-accent"
        label="Save"
        title="Save"
        loading={editSubmitting}
        loadingLabel="Saving…"
        disabled={editSubmitting || !isConnected || !editFormValid}
      />
    </div>
  </form>
</dialog>
