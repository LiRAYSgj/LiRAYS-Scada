<script lang="ts">
	type Props = {
		id: string;
		value: string;
		active: boolean;
		onSelect: (value: string) => void;
	};

	let { id, value, active, onSelect }: Props = $props();
</script>

<button
	{id}
	type="button"
	role="option"
	aria-selected={active}
	class="hover:bg-accent hover:text-accent-foreground aria-selected:bg-accent aria-selected:text-accent-foreground w-full cursor-pointer rounded-sm px-2 py-1.5 text-left text-xs/relaxed outline-none transition-colors select-none"
	onmousedown={(e) => {
		e.preventDefault();
		onSelect(value);
	}}
>
	{value}
</button>
