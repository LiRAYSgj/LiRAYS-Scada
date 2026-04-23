<script lang="ts">
	import { Button } from '$lib/components/Button';
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
	<Button
		variant="ghost"
		title={isExpanded ? 'Collapse node' : 'Expand node'}
		ariaLabel={isExpanded ? 'Collapse node' : 'Expand node'}
		class="h-5 min-w-0 w-5 p-0 text-muted-foreground"
		onclick={(e) => {
			e?.stopPropagation();
			onToggle();
		}}
	>
		{#snippet children()}
			{#if isLoading}
				<LoaderCircle class="h-3.5 w-3.5 animate-spin" />
			{:else if isExpanded}
				<ChevronDown class="h-3.5 w-3.5" />
			{:else}
				<ChevronRight class="h-3.5 w-3.5" />
			{/if}
		{/snippet}
	</Button>
{:else}
	<span class="inline-block h-5 w-5" aria-hidden="true"></span>
{/if}
