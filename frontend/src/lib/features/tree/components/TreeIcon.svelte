<script lang="ts">
	import { Folder, Tag } from 'lucide-svelte';
	import type { TreeNodeKind } from '../types';

	interface Props {
		kind: TreeNodeKind;
	}

	let { kind }: Props = $props();
</script>

{#if kind === 'folder'}
	<Folder class="h-4 w-4 shrink-0 text-amber-600 dark:text-amber-400" aria-hidden="true" />
{:else}
	<Tag class="h-4 w-4 shrink-0 text-sky-700 dark:text-sky-400" aria-hidden="true" />
{/if}
