<script lang="ts">
	import { Button } from "$lib/components/Button";
	import { ButtonGroup } from "$lib/components/ui/button-group";
	import { Button as UIButton } from "$lib/components/ui/button";
	import { Separator } from "$lib/components/ui/separator";
	import * as Avatar from "$lib/components/ui/avatar";
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
		username,
		isAddDisabled,
		multiSelectMode,
		onToggleMultiSelect,
		selectionCount,
		onRemoveSelection,
		onSelectAll,
	}: Props = $props();

	const avatarInitial = $derived(
		username.trim().charAt(0).toUpperCase() || "U",
	);
</script>

<div class="flex items-center justify-between rounded-md border border-border bg-card px-3 py-2 shadow-sm">
	<div class="flex w-[calc(30%-6px)] min-w-[360px] items-center gap-3">
		<h1 class="truncate pr-1 text-base font-semibold text-foreground">Namespace Browser</h1>
		<Separator orientation="vertical" class="h-6" />
		<div class="ml-auto flex items-center gap-1">
			{#if multiSelectMode}
				<Button
					variant="icon"
					icon={Trash2}
					title="Remove selected"
					ariaLabel="Remove selected"
					disabled={selectionCount === 0}
					class="border-destructive/45 text-destructive hover:border-destructive/70 hover:bg-destructive/12 hover:text-destructive"
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
	<div class="ml-3 flex shrink-0 items-center gap-2 rounded-lg border border-border bg-card px-2 py-1.5">
		<ButtonGroup class="gap-0">
			<UIButton
				variant="outline"
				size="sm"
				class="gap-1.5"
				title={canvasMode === "edit" ? "Play" : "Edit"}
				onclick={onToggleCanvasMode}
			>
				{#if canvasMode === "edit"}
					<Play class="text-emerald-500" />
					<span>Play</span>
				{:else}
					<Square class="text-amber-500" />
					<span>Edit</span>
				{/if}
			</UIButton>
			<UIButton
				variant="outline"
				size="sm"
				class="gap-1.5"
				title={theme === "dark" ? "Light mode" : "Dark mode"}
				onclick={onToggleTheme}
			>
				{#if theme === "dark"}
					<Sun />
					<span>Light mode</span>
				{:else}
					<Moon />
					<span>Dark mode</span>
				{/if}
			</UIButton>
		</ButtonGroup>
		<Separator orientation="vertical" class="mx-1 h-6" />
		<div class="flex items-center gap-2">
			<Avatar.Root class="bg-primary text-primary-foreground">
				<Avatar.Fallback>{avatarInitial}</Avatar.Fallback>
			</Avatar.Root>
			<div class="leading-tight">
				<div class="text-sm font-semibold text-foreground">{username}</div>
				<div class="text-xs text-muted-foreground">Signed in</div>
			</div>
		</div>
		<form method="get" action="/auth/logout">
			<UIButton variant="outline" size="sm" class="ml-1" type="submit">Logout</UIButton>
		</form>
	</div>
</div>
