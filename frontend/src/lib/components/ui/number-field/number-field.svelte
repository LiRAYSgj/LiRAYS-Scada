<script lang="ts">
	import MinusIcon from "@lucide/svelte/icons/minus";
	import PlusIcon from "@lucide/svelte/icons/plus";
	import { Input } from "$lib/components/ui/input";
	import { cn } from "$lib/utils.js";
	import type { HTMLInputAttributes } from "svelte/elements";

	type Props = Omit<HTMLInputAttributes, "type" | "value" | "files"> & {
		value?: number;
		step?: number | "any";
		min?: number;
		max?: number;
		class?: string;
	};

	let {
		value = $bindable(undefined),
		step = 1,
		min = undefined,
		max = undefined,
		class: className,
		disabled = false,
		...restProps
	}: Props = $props();

	function clamp(next: number): number {
		if (min !== undefined && next < min) return min;
		if (max !== undefined && next > max) return max;
		return next;
	}

	function updateBy(delta: number): void {
		if (disabled) return;
		const base = value ?? min ?? 0;
		value = clamp(base + delta);
	}

	const numericStep = $derived(
		typeof step === "number" && Number.isFinite(step) ? step : 1,
	);
</script>

<div class="relative w-full">
	<Input
		type="number"
		bind:value
		{min}
		{max}
		{step}
		{disabled}
		data-number-field-input="true"
		class={cn("w-full pr-18", className)}
		{...restProps}
	/>
	<div class="pointer-events-none absolute inset-y-0 right-1 flex items-center">
		<div class="pointer-events-auto flex items-center gap-1">
			<button
				type="button"
				class="inline-flex h-6 w-6 cursor-pointer items-center justify-center rounded-md border border-black/15 bg-(--bg-panel) text-(--text-secondary) transition-colors hover:bg-(--bg-hover) hover:text-(--text-primary) disabled:cursor-not-allowed disabled:opacity-40 dark:border-white/10"
				disabled={disabled}
				tabindex="-1"
				aria-label="Decrease value"
				onclick={() => updateBy(-numericStep)}
			>
				<MinusIcon class="size-3.5" />
			</button>
			<button
				type="button"
				class="inline-flex h-6 w-6 cursor-pointer items-center justify-center rounded-md border border-black/15 bg-(--bg-panel) text-(--text-secondary) transition-colors hover:bg-(--bg-hover) hover:text-(--text-primary) disabled:cursor-not-allowed disabled:opacity-40 dark:border-white/10"
				disabled={disabled}
				tabindex="-1"
				aria-label="Increase value"
				onclick={() => updateBy(numericStep)}
			>
				<PlusIcon class="size-3.5" />
			</button>
		</div>
	</div>
</div>

<style>
	:global(input[data-number-field-input="true"]) {
		appearance: textfield;
		-moz-appearance: textfield;
	}

	:global(input[data-number-field-input="true"]::-webkit-outer-spin-button),
	:global(input[data-number-field-input="true"]::-webkit-inner-spin-button) {
		margin: 0;
		-webkit-appearance: none;
	}
</style>
