<script lang="ts">
	import { Button as UIButton } from "$lib/components/ui/button";
	import { cn } from "$lib/utils.js";
	import type {
		ButtonSize as UiButtonSize,
		ButtonVariant as UiButtonVariant,
	} from "$lib/components/ui/button";

	/** Accepts Svelte components (e.g. Lucide icons). Typed loosely for compatibility. */
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	export type ButtonIcon = any;

	export type ButtonVariant =
		| "icon"
		| "ghost"
		| "outline-muted"
		| "outline-accent"
		| "filled-accent"
		| "filled-warn";

	type PresetButtonConfig = {
		variant: UiButtonVariant;
		size: UiButtonSize;
		className?: string;
	};

	let {
		variant = "outline-muted",
		type = "button",
		disabled = false,
		loading = false,
		selected = false,
		icon = undefined as ButtonIcon | undefined,
		label = "",
		loadingLabel = "Loading…",
		iconClass = "",
		title = "",
		ariaLabel = "",
		class: className = "",
		style = "",
		onclick = undefined as ((e?: MouseEvent) => void) | undefined,
		ondblclick = undefined as (() => void) | undefined,
		onmouseenter = undefined as (() => void) | undefined,
		onpointerdown = undefined as ((e: PointerEvent) => void) | undefined,
		children = undefined as import("svelte").Snippet | undefined,
	}: {
		variant?: ButtonVariant;
		type?: "button" | "submit";
		disabled?: boolean;
		loading?: boolean;
		/** When true with outline-muted or ghost, renders as filled-accent (selected state). */
		selected?: boolean;
		icon?: ButtonIcon;
		label?: string;
		loadingLabel?: string;
		iconClass?: string;
		title?: string;
		ariaLabel?: string;
		class?: string;
		style?: string;
		onclick?: (e?: MouseEvent) => void;
		ondblclick?: () => void;
		onmouseenter?: () => void;
		onpointerdown?: (e: PointerEvent) => void;
		children?: import("svelte").Snippet;
	} = $props();

	const effectiveVariant = $derived(
		selected && (variant === "outline-muted" || variant === "ghost")
			? "filled-accent"
			: variant,
	);
	const resolvedAriaLabel = $derived(ariaLabel || title || label || undefined);

	const presetConfig = $derived.by<PresetButtonConfig>(() => {
		switch (effectiveVariant) {
			case "icon":
				return { variant: "outline", size: "icon-sm" };
			case "ghost":
				return { variant: "ghost", size: "sm" };
			case "outline-accent":
				return {
					variant: "outline",
					size: "sm",
					className: "border-primary/40 text-primary hover:border-primary/60 hover:bg-primary/10",
				};
			case "filled-accent":
				return { variant: "default", size: "sm" };
			case "filled-warn":
				return { variant: "destructive", size: "sm" };
			case "outline-muted":
			default:
				return {
					variant: "outline",
					size: "sm",
					className: "border-primary/35 bg-primary/6 text-foreground hover:border-primary/55 hover:bg-primary/14",
				};
		}
	});
</script>

<UIButton
	{type}
	variant={presetConfig.variant}
	size={presetConfig.size}
	class={cn("gap-1.5", presetConfig.className, className)}
	style={style}
	disabled={disabled || loading}
	{title}
	aria-label={resolvedAriaLabel}
	aria-busy={loading}
	onclick={(e) => {
		if (onclick && !disabled && !loading) onclick(e);
	}}
	ondblclick={() => {
		if (ondblclick && !disabled && !loading) ondblclick();
	}}
	onmouseenter={onmouseenter}
	onpointerdown={onpointerdown}
>
	{#if loading}
		<span class="btn__spinner" aria-hidden="true"></span>
		{#if label || loadingLabel}
			<span class="whitespace-nowrap">{loadingLabel}</span>
		{/if}
	{:else if children}
		{@render children()}
	{:else}
		{#if icon}
			{@const Icon = icon}
			<span class={cn("inline-flex shrink-0 items-center justify-center", iconClass)} aria-hidden="true">
				<Icon />
			</span>
		{/if}
		{#if label}
			<span class="whitespace-nowrap">{label}</span>
		{/if}
	{/if}
</UIButton>

<style>
	.btn__spinner {
		display: inline-block;
		width: 0.75rem;
		height: 0.75rem;
		border: 2px solid currentColor;
		border-right-color: transparent;
		border-radius: 999px;
		animation: btn-spin 0.6s linear infinite;
	}

	:global(.btn--align-start) {
		justify-content: flex-start;
		text-align: left;
	}

	@keyframes btn-spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
