<script lang="ts">
	import { Button } from "$lib/components/Button";
	import { CheckSquare, Layers, ListChecks, Moon, Play, Plus, Square, Sun, Trash2 } from "lucide-svelte";

	type ThemeMode = "light" | "dark";
	type CanvasMode = "edit" | "play";

	interface Props {
		theme: ThemeMode;
		canvasMode: CanvasMode;
		username: string;
		onToggleCanvasMode: () => void;
		onToggleTheme: () => void;
		onOpenAddDialog: () => void;
		onOpenNamespaceBuilder: () => void;
		onLogout: () => void;
		isAddDisabled: boolean;
		/** When true, Add and Namespace Builder are hidden and Remove selection is shown. */
		multiSelectMode: boolean;
		onToggleMultiSelect: () => void;
		/** Number of nodes in the multi-selection (when multiSelectMode is true). */
		selectionCount: number;
		onRemoveSelection: () => void;
		onSelectAll: () => void;
	}

	let {
		theme,
		canvasMode,
		onToggleCanvasMode,
		onToggleTheme,
		onOpenAddDialog,
		onOpenNamespaceBuilder,
		onLogout,
		username,
		isAddDisabled,
		multiSelectMode,
		onToggleMultiSelect,
		selectionCount,
		onRemoveSelection,
		onSelectAll,
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
					icon={CheckSquare}
					title="Select all"
					ariaLabel="Select all"
					onclick={onSelectAll}
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
	<div class="flex items-center gap-3">
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
		<div class="flex items-center gap-2 rounded-xl border border-(--surface-border) bg-(--surface-2) px-3 py-2 shadow-sm">
			<div
				class="flex h-9 w-9 items-center justify-center rounded-full text-sm font-semibold text-white"
				style="background: linear-gradient(135deg, #38bdf8, #6366f1);"
				title="Signed in user"
			>
				{username.slice(0, 1).toUpperCase()}
			</div>
			<div class="leading-tight">
				<div class="text-sm font-semibold text-(--text-primary)">{username}</div>
				<div class="text-xs text-(--text-muted)">Signed in</div>
			</div>
			<Button variant="outline-muted" label="Logout" onclick={onLogout} />
		</div>
	</div>
</div>
