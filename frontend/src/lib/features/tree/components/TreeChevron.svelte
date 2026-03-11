<script lang="ts">
	import { ChevronDown, ChevronRight, LoaderCircle } from 'lucide-svelte';

	interface Props {
		hasChildren: boolean;
		isExpanded: boolean;
		isLoading: boolean;
		onToggle: () => void;
	}

	let { hasChildren, isExpanded, isLoading, onToggle }: Props = $props();
</script>

{#if hasChildren}
	<button
		type="button"
		class="inline-flex h-5 w-5 cursor-pointer items-center justify-center rounded text-(--text-muted) hover:bg-(--bg-hover) hover:text-(--text-primary)"
		onclick={(event) => {
			event.stopPropagation();
			onToggle();
		}}
		aria-label={isExpanded ? 'Collapse node' : 'Expand node'}
	>
		{#if isLoading}
			<LoaderCircle class="h-3.5 w-3.5 animate-spin" />
		{:else if isExpanded}
			<ChevronDown class="h-3.5 w-3.5" />
		{:else}
			<ChevronRight class="h-3.5 w-3.5" />
		{/if}
	</button>
{:else}
	<span class="inline-block h-5 w-5" aria-hidden="true"></span>
{/if}
