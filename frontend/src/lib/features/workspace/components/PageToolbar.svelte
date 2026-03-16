<script lang="ts">
	import { Button } from "$lib/components/Button";
	import { Layers, ListChecks, Moon, Play, Plus, Square, Sun, Trash2 } from "lucide-svelte";

	type ThemeMode = "light" | "dark";
	type CanvasMode = "edit" | "play";

	interface Props {
		theme: ThemeMode;
		canvasMode: CanvasMode;
		onToggleCanvasMode: () => void;
		onToggleTheme: () => void;
		onOpenAddDialog: () => void;
		onOpenNamespaceBuilder: () => void;
		isAddDisabled: boolean;
		/** When true, Add and Namespace Builder are hidden and Remove selection is shown. */
		multiSelectMode: boolean;
		onToggleMultiSelect: () => void;
		/** Number of nodes in the multi-selection (when multiSelectMode is true). */
		selectionCount: number;
		onRemoveSelection: () => void;
	}

	let {
		theme,
		canvasMode,
		onToggleCanvasMode,
		onToggleTheme,
		onOpenAddDialog,
		onOpenNamespaceBuilder,
		isAddDisabled,
		multiSelectMode,
		onToggleMultiSelect,
		selectionCount,
		onRemoveSelection,
	}: Props = $props();
</script>

<div class="mb-3 flex items-center justify-between">
	<div class="flex w-[30%] min-w-[360px] items-center justify-between">
		<h1 class="truncate pr-2 text-base font-semibold text-(--text-primary)">Namespace Browser</h1>
		<div class="flex items-center gap-1">
			{#if multiSelectMode}
				<Button
					variant="icon"
					icon={Trash2}
					title="Remove selected"
					ariaLabel="Remove selected"
					disabled={selectionCount === 0}
					onclick={onRemoveSelection}
				/>
				<Button
					variant="icon"
					icon={ListChecks}
					title="Multi-selection mode (click to exit)"
					ariaLabel="Multi-selection mode"
					selected={true}
					onclick={onToggleMultiSelect}
				/>
			{:else}
				<Button
					variant="icon"
					icon={Plus}
					title="Add variable or folder"
					ariaLabel="Add variable or folder"
					disabled={isAddDisabled}
					onclick={onOpenAddDialog}
				/>
				<Button
					variant="icon"
					icon={Layers}
					title="Namespace Template Builder"
					ariaLabel="Namespace Template Builder"
					onclick={onOpenNamespaceBuilder}
				/>
				<Button
					variant="icon"
					icon={ListChecks}
					title="Multi-selection mode"
					ariaLabel="Multi-selection mode"
					onclick={onToggleMultiSelect}
				/>
			{/if}
		</div>
	</div>
	<div class="flex items-center gap-2">
		<Button
			variant="outline-muted"
			icon={canvasMode === "edit" ? Play : Square}
			label={canvasMode === "edit" ? "Play" : "Edit"}
			title={canvasMode === "edit" ? "Play" : "Edit"}
			iconClass={canvasMode === "edit" ? "text-emerald-500" : "text-amber-500"}
			onclick={onToggleCanvasMode}
		/>
		<Button
			variant="outline-muted"
			icon={theme === "dark" ? Sun : Moon}
			label={theme === "dark" ? "Light mode" : "Dark mode"}
			title={theme === "dark" ? "Light mode" : "Dark mode"}
			onclick={onToggleTheme}
		/>
	</div>
</div>
