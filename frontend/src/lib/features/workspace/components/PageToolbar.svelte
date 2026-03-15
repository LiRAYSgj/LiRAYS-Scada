<script lang="ts">
	import { Button } from "$lib/components/Button";
	import { Layers, Moon, Play, Plus, Square, Sun } from "lucide-svelte";

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
	}

	let { theme, canvasMode, onToggleCanvasMode, onToggleTheme, onOpenAddDialog, onOpenNamespaceBuilder, isAddDisabled }: Props =
		$props();
</script>

<div class="mb-3 flex items-center justify-between">
	<div class="flex w-[30%] min-w-[360px] items-center justify-between">
		<h1 class="truncate pr-2 text-base font-semibold text-(--text-primary)">Namespace Browser</h1>
		<div class="flex items-center gap-1">
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
		</div>
	</div>
	<div class="flex items-center gap-2">
		<Button
			variant="outline-muted"
			icon={canvasMode === "edit" ? Play : Square}
			label={canvasMode === "edit" ? "Play" : "Edit"}
			title={canvasMode === "edit" ? "Play" : "Edit"}
			onclick={onToggleCanvasMode}
			class={canvasMode === "edit" ? "text-emerald-500" : "text-amber-500"}
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
