<script lang="ts">
	import { Moon, Play, Square, Sun } from 'lucide-svelte';

	type ThemeMode = 'light' | 'dark';
	type CanvasMode = 'edit' | 'play';

	interface Props {
		theme: ThemeMode;
		canvasMode: CanvasMode;
		onToggleCanvasMode: () => void;
		onToggleTheme: () => void;
		onOpenAddDialog: () => void;
		isAddDisabled: boolean;
	}

	let { theme, canvasMode, onToggleCanvasMode, onToggleTheme, onOpenAddDialog, isAddDisabled }: Props =
		$props();
</script>

<div class="mb-3 flex items-center justify-between">
	<div class="flex w-[30%] min-w-[360px] items-center justify-between">
		<h1 class="truncate pr-2 text-base font-semibold text-(--text-primary)">Namespace Browser</h1>
		<button
			type="button"
			class="inline-flex h-7 w-7 cursor-pointer items-center justify-center rounded border border-black/10 bg-(--bg-panel) text-(--text-secondary) hover:bg-(--bg-hover) disabled:cursor-not-allowed disabled:opacity-50 dark:border-white/10"
			style="background-color: var(--bg-panel);"
			disabled={isAddDisabled}
			onclick={onOpenAddDialog}
			aria-label="Add item"
			title="Add item"
		>
			+
		</button>
	</div>
	<div class="flex items-center gap-2">
		<button
			type="button"
			class="inline-flex cursor-pointer items-center gap-2 rounded border border-black/10 bg-(--bg-panel) px-3 py-1.5 text-xs text-(--text-secondary) hover:bg-(--bg-hover) dark:border-white/10"
			style="background-color: var(--bg-panel);"
			onclick={onToggleCanvasMode}
		>
			{#if canvasMode === 'edit'}
				<Play class="h-4 w-4 text-emerald-500" />
				<span>Play</span>
			{:else}
				<Square class="h-4 w-4 text-amber-500" />
				<span>Edit</span>
			{/if}
		</button>
		<button
			type="button"
			class="inline-flex cursor-pointer items-center gap-2 rounded border border-black/10 bg-(--bg-panel) px-3 py-1.5 text-xs text-(--text-secondary) hover:bg-(--bg-hover) dark:border-white/10"
			style="background-color: var(--bg-panel);"
			onclick={onToggleTheme}
		>
			{#if theme === 'dark'}
				<Sun class="h-4 w-4" />
				<span>Light mode</span>
			{:else}
				<Moon class="h-4 w-4" />
				<span>Dark mode</span>
			{/if}
		</button>
	</div>
</div>
