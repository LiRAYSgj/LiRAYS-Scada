<script lang="ts">
	import type { HTMLInputAttributes, HTMLInputTypeAttribute } from "svelte/elements";
	import { cn, type WithElementRef } from "$lib/utils.js";

	type InputType = Exclude<HTMLInputTypeAttribute, "file">;

	type Props = WithElementRef<
		Omit<HTMLInputAttributes, "type"> &
			({ type: "file"; files?: FileList } | { type?: InputType; files?: undefined })
	>;

	let {
		ref = $bindable(null),
		value = $bindable(),
		type,
		files = $bindable(),
		class: className,
		"data-slot": dataSlot = "input",
		...restProps
	}: Props = $props();
</script>

{#if type === "file"}
	<input
		bind:this={ref}
		data-slot={dataSlot}
		class={cn(
			"border border-black/15 bg-(--bg-muted) text-(--text-primary) focus-visible:border-blue-500 focus-visible:ring-1 focus-visible:ring-blue-500/35 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive dark:aria-invalid:border-destructive/50 disabled:bg-(--bg-muted) h-8 rounded-lg px-2.5 py-1 text-base transition-colors file:h-6 file:text-sm file:font-medium aria-invalid:ring-1 md:text-sm file:text-(--text-primary) placeholder:text-muted-foreground w-full min-w-0 outline-none file:inline-flex file:border-0 file:bg-transparent disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 dark:border-white/10",
			className
		)}
		type="file"
		bind:files
		bind:value
		{...restProps}
	/>
{:else}
	<input
		bind:this={ref}
		data-slot={dataSlot}
		class={cn(
			"border border-black/15 bg-(--bg-muted) text-(--text-primary) focus-visible:border-blue-500 focus-visible:ring-1 focus-visible:ring-blue-500/35 aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive dark:aria-invalid:border-destructive/50 disabled:bg-(--bg-muted) h-8 rounded-lg px-2.5 py-1 text-base transition-colors file:h-6 file:text-sm file:font-medium aria-invalid:ring-1 md:text-sm file:text-(--text-primary) placeholder:text-muted-foreground w-full min-w-0 outline-none file:inline-flex file:border-0 file:bg-transparent disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 dark:border-white/10",
			className
		)}
		{type}
		bind:value
		{...restProps}
	/>
{/if}
