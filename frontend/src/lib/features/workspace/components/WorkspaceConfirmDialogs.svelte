<script lang="ts">
  import { Button } from "$lib/components/Button";
  import * as Dialog from "$lib/components/ui/dialog";
  import type { ScadaView } from "$lib/features/views/types";
  import type { TreeNode } from "$lib/features/tree/types";
  import type { WebSocketConnectionStatus } from "$lib/core/ws/types";

  interface Props {
    removeViewDialogOpen: boolean;
    removeViewTarget: ScadaView | null;
    removeViewSubmitting: boolean;
    removeViewError: string;
    onRemoveViewDialogOpenChange: (open: boolean) => void;
    onConfirmRemoveView: () => void;
    onCloseRemoveViewDialog: () => void;

    removeDialogOpen: boolean;
    removeTargetNode: TreeNode | null;
    removeSubmitting: boolean;
    removeError: string;
    wsStatus: WebSocketConnectionStatus;
    onRemoveDialogOpenChange: (open: boolean) => void;
    onConfirmRemoveNode: () => void;
    onCloseRemoveDialog: () => void;

    removeMultipleDialogOpen: boolean;
    removeMultipleSubmitting: boolean;
    removeMultipleError: string;
    onRemoveMultipleDialogOpenChange: (open: boolean) => void;
    onConfirmRemoveMultiple: () => void;
    onCloseRemoveMultipleDialog: () => void;

    leaveViewDialogOpen: boolean;
    activeViewName: string | null;
    routeViewId: string | null;
    leaveViewSubmitting: boolean;
    canConfirmLeave: boolean;
    onLeaveViewDialogOpenChange: (open: boolean) => void;
    onConfirmLeaveView: () => void;
    onCloseLeaveViewDialog: () => void;
  }

  let {
    removeViewDialogOpen,
    removeViewTarget,
    removeViewSubmitting,
    removeViewError,
    onRemoveViewDialogOpenChange,
    onConfirmRemoveView,
    onCloseRemoveViewDialog,

    removeDialogOpen,
    removeTargetNode,
    removeSubmitting,
    removeError,
    wsStatus,
    onRemoveDialogOpenChange,
    onConfirmRemoveNode,
    onCloseRemoveDialog,

    removeMultipleDialogOpen,
    removeMultipleSubmitting,
    removeMultipleError,
    onRemoveMultipleDialogOpenChange,
    onConfirmRemoveMultiple,
    onCloseRemoveMultipleDialog,

    leaveViewDialogOpen,
    activeViewName,
    routeViewId,
    leaveViewSubmitting,
    canConfirmLeave,
    onLeaveViewDialogOpenChange,
    onConfirmLeaveView,
    onCloseLeaveViewDialog,
  }: Props = $props();
</script>

<Dialog.Root
  open={removeViewDialogOpen}
  onOpenChange={onRemoveViewDialogOpenChange}
>
  <Dialog.Content
    class="max-w-[420px]"
    showCloseButton={false}
    onInteractOutside={(event) => {
      event.preventDefault();
    }}
    onEscapeKeydown={(event) => {
      event.preventDefault();
    }}
  >
    <form
      class="flex flex-col gap-4"
      onsubmit={(event) => {
        event.preventDefault();
        onConfirmRemoveView();
      }}
    >
      <Dialog.Header>
        <Dialog.Title>Confirm removal</Dialog.Title>
        <Dialog.Description>
          Remove view "{removeViewTarget?.name}"? This action cannot be undone.
        </Dialog.Description>
        {#if removeViewError}
          <p class="text-destructive text-xs/relaxed">{removeViewError}</p>
        {/if}
      </Dialog.Header>
      <Dialog.Footer class="border-border border-t pt-4">
        <Button
          variant="outline-muted"
          label="Cancel"
          title="Cancel"
          disabled={removeViewSubmitting}
          onclick={onCloseRemoveViewDialog}
        />
        <Button
          type="submit"
          variant="filled-warn"
          label="Remove"
          loadingLabel="Removing..."
          loading={removeViewSubmitting}
          disabled={removeViewSubmitting || !removeViewTarget}
        />
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>

<Dialog.Root open={removeDialogOpen} onOpenChange={onRemoveDialogOpenChange}>
  <Dialog.Content
    class="max-w-[420px]"
    showCloseButton={false}
    onInteractOutside={(event) => {
      event.preventDefault();
    }}
    onEscapeKeydown={(event) => {
      event.preventDefault();
    }}
  >
    <form
      class="flex flex-col gap-4"
      onsubmit={(event) => {
        event.preventDefault();
        onConfirmRemoveNode();
      }}
    >
      <Dialog.Header>
        <Dialog.Title>Confirm removal</Dialog.Title>
        <Dialog.Description>
          Remove "{removeTargetNode?.name}" ({removeTargetNode?.kind === "folder"
            ? "folder"
            : "variable"})? This action cannot be undone.
        </Dialog.Description>
        {#if removeError}
          <p class="text-destructive text-xs/relaxed">{removeError}</p>
        {/if}
      </Dialog.Header>
      <Dialog.Footer class="border-border border-t pt-4">
        <Button
          variant="outline-muted"
          label="Cancel"
          title="Cancel"
          disabled={removeSubmitting}
          onclick={onCloseRemoveDialog}
        />
        <Button
          type="submit"
          variant="filled-warn"
          label="Remove"
          loadingLabel="Removing..."
          loading={removeSubmitting}
          disabled={removeSubmitting || wsStatus !== "connected" || !removeTargetNode}
        />
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>

<Dialog.Root
  open={removeMultipleDialogOpen}
  onOpenChange={onRemoveMultipleDialogOpenChange}
>
  <Dialog.Content
    class="max-w-[420px]"
    showCloseButton={false}
    onInteractOutside={(event) => {
      event.preventDefault();
    }}
    onEscapeKeydown={(event) => {
      event.preventDefault();
    }}
  >
    <form
      class="flex flex-col gap-4"
      onsubmit={(event) => {
        event.preventDefault();
        onConfirmRemoveMultiple();
      }}
    >
      <Dialog.Header>
        <Dialog.Title>Remove selection</Dialog.Title>
        <Dialog.Description>
          Remove selected item(s)? All descendants will also be removed. This action
          cannot be undone.
        </Dialog.Description>
        {#if removeMultipleError}
          <p class="text-destructive text-xs/relaxed">{removeMultipleError}</p>
        {/if}
      </Dialog.Header>
      <Dialog.Footer class="border-border border-t pt-4">
        <Button
          variant="outline-muted"
          label="Cancel"
          title="Cancel"
          disabled={removeMultipleSubmitting}
          onclick={onCloseRemoveMultipleDialog}
        />
        <Button
          type="submit"
          variant="filled-warn"
          label="Remove"
          loadingLabel="Removing..."
          loading={removeMultipleSubmitting}
          disabled={removeMultipleSubmitting || wsStatus !== "connected"}
        />
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>

<Dialog.Root open={leaveViewDialogOpen} onOpenChange={onLeaveViewDialogOpenChange}>
  <Dialog.Content
    class="max-w-[420px]"
    showCloseButton={false}
    onInteractOutside={(event) => {
      event.preventDefault();
    }}
    onEscapeKeydown={(event) => {
      event.preventDefault();
    }}
  >
    <form
      class="flex flex-col gap-4"
      onsubmit={(event) => {
        event.preventDefault();
        onConfirmLeaveView();
      }}
    >
      <Dialog.Header>
        <Dialog.Title>Leave current view?</Dialog.Title>
        <Dialog.Description>
          You are about to leave "{activeViewName ?? routeViewId}". Any unsaved graph
          changes will be lost.
        </Dialog.Description>
      </Dialog.Header>
      <Dialog.Footer class="border-border border-t pt-4">
        <Button
          variant="outline-muted"
          label="Stay"
          title="Stay"
          disabled={leaveViewSubmitting}
          onclick={onCloseLeaveViewDialog}
        />
        <Button
          type="submit"
          variant="filled-warn"
          label="Leave view"
          loadingLabel="Leaving..."
          loading={leaveViewSubmitting}
          disabled={leaveViewSubmitting || !canConfirmLeave}
        />
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>
