<script lang="ts">
	import XIcon from '@lucide/svelte/icons/x';

	type Props = {
		value: string;
		disabled: boolean | null;
		active: boolean;
		onDelete: (value: string) => void;
	};

	let { value, disabled, onDelete, active }: Props = $props();
</script>

<div
	class="bg-(--bg-muted) text-(--text-primary) ring-offset-background hover:bg-(--bg-muted) aria-selected:ring-ring flex place-items-center gap-2 rounded-md border border-black/10 px-2 py-0.5 transition-all aria-selected:ring-1 aria-selected:ring-blue-500/35 dark:border-white/10"
	aria-selected={active}
>
	<span>
		{value}
	</span>
	<button type="button" {disabled} onclick={() => onDelete(value)} class="cursor-pointer">
		<XIcon class="size-4" />
	</button>
</div>
