<script lang="ts">
  import { Button } from "$lib/components/Button";
  import { ArrowLeft, BugPlay, Flag, Save, Square } from "lucide-svelte";
  import type { CanvasMode, ScadaView } from "../types";

  interface Props {
    view: ScadaView | null;
    canvasMode: CanvasMode;
    saving?: boolean;
    onBackToViewsList: () => void;
    onToggleCanvasMode: () => void;
    onSave: () => void;
    onSetEntryPoint: () => void;
  }

  let {
    view,
    canvasMode,
    saving = false,
    onBackToViewsList,
    onToggleCanvasMode,
    onSave,
    onSetEntryPoint,
  }: Props = $props();
</script>

<header class="flex items-center justify-between border-b border-border px-4 py-2.5">
  <div class="flex items-center gap-2">
    <Button
      variant="icon"
      icon={ArrowLeft}
      label=""
      title="Back to views"
      ariaLabel="Back to views list"
      onclick={onBackToViewsList}
    />
    <div>
      <div class="text-sm font-semibold text-foreground">{view?.name ?? "Untitled view"}</div>
      <div class="text-xs text-muted-foreground">View Editor</div>
    </div>
  </div>

  <div class="flex items-center gap-1">
    <Button
      variant={canvasMode === "edit" ? "outline-accent" : "outline-muted"}
      icon={canvasMode === "edit" ? BugPlay : Square}
      label={canvasMode === "edit" ? "Play" : "Stop"}
      title={canvasMode === "edit" ? "Switch to play mode" : "Stop play mode"}
      class={canvasMode === "edit"
        ? ""
        : "border-destructive/45 text-destructive hover:border-destructive/70 hover:bg-destructive/12 hover:text-destructive"}
      onclick={onToggleCanvasMode}
    />
    <Button
      variant="outline-muted"
      icon={Flag}
      label={view?.is_entry_point ? "Entry point" : "Set entry point"}
      title="Set entry point"
      disabled={!view || !!view?.is_entry_point}
      onclick={onSetEntryPoint}
    />
    <Button
      variant="filled-accent"
      icon={Save}
      label="Save"
      title="Save view"
      loading={saving}
      loadingLabel="Saving..."
      disabled={!view || saving}
      onclick={onSave}
    />
  </div>
</header>
