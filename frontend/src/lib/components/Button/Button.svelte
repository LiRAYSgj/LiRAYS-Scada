<script lang="ts">
	import { Button as UIButton } from "$lib/components/ui/button";
	import { cn } from "$lib/utils.js";
	import type {
		ButtonProps as UiButtonProps,
		ButtonSize as UiButtonSize,
		ButtonVariant as UiButtonVariant,
	} from "$lib/components/ui/button";

	/** Accepts Svelte components (e.g. Lucide icons). Typed loosely for compatibility. */
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	export type ButtonIcon = any;

	type AppPresetVariant =
		| "icon"
		| "outline-muted"
		| "outline-accent"
		| "filled-accent"
		| "filled-warn";

	export type ButtonVariant = UiButtonVariant | AppPresetVariant;

	export type ButtonProps = Omit<
		UiButtonProps,
		"variant" | "size" | "children" | "class"
	> & {
		variant?: ButtonVariant;
		size?: UiButtonSize;
		/** Visual selected state for tab-like controls. */
		selected?: boolean;
		/** Shows spinner and disables interaction. */
		loading?: boolean;
		icon?: ButtonIcon;
		label?: string;
		loadingLabel?: string;
		iconClass?: string;
		/** Convenience alias for `aria-label`. */
		ariaLabel?: string;
		class?: string;
		children?: import("svelte").Snippet;
	};

	type PresetButtonConfig = {
		variant: UiButtonVariant;
		defaultSize?: UiButtonSize;
		className?: string;
	};

	let {
		variant = "outline-muted",
		size = undefined,
		disabled = false,
		loading = false,
		selected = false,
		icon = undefined as ButtonIcon | undefined,
		label = "",
		loadingLabel = "Loading…",
		iconClass = "",
		ariaLabel = "",
		class: className = "",
		children = undefined as import("svelte").Snippet | undefined,
		...restProps
	}: ButtonProps = $props();

	const effectiveVariant = $derived(
		selected && (variant === "outline-muted" || variant === "ghost")
			? "filled-accent"
			: variant,
	);
	const resolvedAriaLabel = $derived(
		ariaLabel ||
			(typeof restProps["aria-label"] === "string" ? restProps["aria-label"] : undefined) ||
			label ||
			undefined,
	);

	const presetConfig = $derived.by<PresetButtonConfig>(() => {
		switch (effectiveVariant) {
			case "icon":
				return { variant: "outline", defaultSize: "icon-sm" };
			case "ghost":
				return { variant: "ghost", defaultSize: "sm" };
			case "outline-accent":
				return {
					variant: "outline",
					defaultSize: "sm",
					className: "border-primary/40 text-primary hover:border-primary/60 hover:bg-primary/10",
				};
			case "filled-accent":
				return { variant: "default", defaultSize: "sm" };
			case "filled-warn":
				return { variant: "destructive", defaultSize: "sm" };
			case "outline-muted":
				return {
					variant: "outline",
					defaultSize: "sm",
					className: "border-primary/35 bg-primary/6 text-foreground hover:border-primary/55 hover:bg-primary/14",
				};
			default:
				return { variant: effectiveVariant };
		}
	});

	const resolvedSize = $derived(size ?? presetConfig.defaultSize ?? "default");
	const resolvedDisabled = $derived(disabled || loading);
</script>

<UIButton
	variant={presetConfig.variant}
	size={resolvedSize}
	class={cn("gap-1.5", presetConfig.className, className)}
	disabled={resolvedDisabled}
	aria-label={resolvedAriaLabel}
	aria-busy={loading}
	{...restProps}
>
	{#if loading}
		<span class="btn__spinner" aria-hidden="true"></span>
		<span class="whitespace-nowrap">{loadingLabel}</span>
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
