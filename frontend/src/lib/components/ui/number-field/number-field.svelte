<script lang="ts">
	import MinusIcon from "@lucide/svelte/icons/minus";
	import PlusIcon from "@lucide/svelte/icons/plus";
	import { Button } from "$lib/components/ui/button";
	import { Input } from "$lib/components/ui/input";
	import { cn } from "$lib/utils.js";
	import type { HTMLInputAttributes } from "svelte/elements";

	type Props = Omit<HTMLInputAttributes, "type" | "value" | "files"> & {
		value?: number;
		step?: number | "any";
		min?: number;
		max?: number;
		class?: string;
		onValueChange?: (value: number | undefined) => void;
	};

	let {
		value = $bindable(undefined),
		step = 1,
		min = undefined,
		max = undefined,
		class: className,
		disabled = false,
		onValueChange,
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
		onValueChange?.(value);
	}

	const numericStep = $derived(
		typeof step === "number" && Number.isFinite(step) ? step : 1,
	);

	function handleInput(): void {
		onValueChange?.(value);
	}
</script>

<div class={cn("relative w-full", className)}>
	<Input
		type="number"
		bind:value
		{min}
		{max}
		{step}
		{disabled}
		data-number-field-input="true"
		class={cn("w-full pr-[2.6rem]", className)}
		oninput={handleInput}
		{...restProps}
	/>
	<div class="pointer-events-none absolute inset-y-0 right-1 z-10 flex items-center">
		<div class="pointer-events-auto flex items-center gap-1">
			<Button
				type="button"
				variant="outline"
				size="icon-xs"
				class="border-border bg-background/80 text-foreground shadow-sm hover:bg-accent"
				disabled={disabled}
				tabindex={-1}
				aria-label="Decrease value"
				onclick={() => updateBy(-numericStep)}
			>
				<MinusIcon />
			</Button>
			<Button
				type="button"
				variant="outline"
				size="icon-xs"
				class="border-border bg-background/80 text-foreground shadow-sm hover:bg-accent"
				disabled={disabled}
				tabindex={-1}
				aria-label="Increase value"
				onclick={() => updateBy(numericStep)}
			>
				<PlusIcon />
			</Button>
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
