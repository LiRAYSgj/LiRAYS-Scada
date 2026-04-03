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
	class="bg-secondary text-secondary-foreground border-border ring-offset-background hover:bg-muted aria-selected:ring-ring flex place-items-center gap-2 rounded-md border px-2 py-0.5 text-xs/relaxed transition-all aria-selected:ring-1"
	aria-selected={active}
>
	<span>
		{value}
	</span>
	<button
		type="button"
		{disabled}
		onclick={() => onDelete(value)}
		class="text-muted-foreground hover:text-foreground inline-flex cursor-pointer items-center justify-center rounded-sm p-0.5 transition-colors disabled:cursor-not-allowed disabled:opacity-50"
	>
		<XIcon class="size-3.5" />
	</button>
</div>
