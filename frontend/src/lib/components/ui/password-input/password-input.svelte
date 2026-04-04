<script lang="ts">
	import EyeIcon from "@lucide/svelte/icons/eye";
	import EyeOffIcon from "@lucide/svelte/icons/eye-off";
	import { Input } from "$lib/components/ui/input";
	import { cn } from "$lib/utils.js";
	import type { HTMLInputAttributes } from "svelte/elements";

	type Props = Omit<HTMLInputAttributes, "type" | "value" | "files"> & {
		value?: string;
		class?: string;
	};

	let {
		value = $bindable(""),
		class: className,
		disabled = false,
		...restProps
	}: Props = $props();

	let visible = $state(false);

	function toggleVisibility(): void {
		if (disabled) return;
		visible = !visible;
	}
</script>

<div
	role="group"
	data-slot="input-group"
	class={cn("flex w-full items-stretch", className)}
>
	<Input
		type={visible ? "text" : "password"}
		bind:value
		{disabled}
		data-slot="input-group-control"
		class="w-full rounded-r-none border-r-0"
		{...restProps}
	/>
	<div
		role="group"
		data-slot="input-group-addon"
		data-align="inline-end"
		class="cn-input-group-addon cn-input-group-addon-align-inline-end bg-input/20 dark:bg-input/30 border-input text-muted-foreground flex select-none items-center justify-center border rounded-r-md border-l-0 px-1.5 order-last"
	>
		<button
			type="button"
			class="cn-input-group-text hover:text-foreground inline-flex items-center justify-center rounded-sm p-0.5 outline-none transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
			disabled={disabled}
			onclick={toggleVisibility}
			aria-label={visible ? "Hide password" : "Show password"}
			title={visible ? "Hide password" : "Show password"}
		>
			{#if visible}
				<EyeOffIcon class="size-4" />
			{:else}
				<EyeIcon class="size-4" />
			{/if}
		</button>
	</div>
</div>
