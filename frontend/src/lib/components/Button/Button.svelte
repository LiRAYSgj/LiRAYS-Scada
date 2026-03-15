<script lang="ts">
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
		/** When true with outline-muted, renders as filled-accent (e.g. tab selected). */
		selected?: boolean;
		icon?: ButtonIcon;
		label?: string;
		loadingLabel?: string;
		/** Optional Tailwind/custom class for the icon (e.g. text-emerald-500). Applied to the icon wrapper so the icon can have a different color than the label. */
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
</script>

<button
	{type}
	class="btn btn--{effectiveVariant} {className}"
	style={style}
	class:btn--icon-only={effectiveVariant === "icon" && !label && !children}
	class:btn--ghost={effectiveVariant === "ghost"}
	{disabled}
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
			<span class="btn__label">{loadingLabel}</span>
		{/if}
	{:else if children}
		{@render children()}
	{:else}
		{#if icon}
			{@const Icon = icon}
			<span class="btn__icon {iconClass}" aria-hidden="true">
				<Icon />
			</span>
		{/if}
		{#if label}
			<span class="btn__label">{label}</span>
		{/if}
	{/if}
</button>

<style>
	.btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		cursor: pointer;
		font-size: 0.75rem;
		line-height: 1.25;
		border-radius: 0.375rem;
		border: 1px solid transparent;
		transition: background-color 0.15s, border-color 0.15s, color 0.15s;
		box-sizing: border-box;
	}
	.btn:disabled {
		cursor: not-allowed;
		opacity: 0.5;
	}
	.btn--icon {
		min-width: 1.75rem;
		height: 1.75rem;
	}
	.btn--icon-only {
		padding: 0;
	}
	.btn--icon:not(.btn--icon-only) {
		padding: 0 0.25rem;
	}
	.btn:not(.btn--icon):not(.btn--icon-only):not(.btn--ghost) {
		padding: 0.375rem 0.625rem;
	}

	/* Ghost: no border, transparent bg (e.g. inline text-style trigger) */
	.btn--ghost {
		background: transparent;
		border-color: transparent;
		color: var(--text-primary);
	}
	.btn--ghost:hover:not(:disabled) {
		background-color: var(--bg-hover);
	}

	/* Icon variant: square, muted outline */
	.btn--icon {
		background-color: var(--bg-panel);
		color: var(--text-secondary);
		border-color: color-mix(in srgb, var(--text-muted) 28%, transparent);
	}
	.btn--icon:hover:not(:disabled) {
		background-color: var(--bg-hover);
	}

	/* Greyed outline for cancel / general actions */
	.btn--outline-muted {
		background-color: var(--bg-panel);
		color: var(--text-secondary);
		border-color: color-mix(in srgb, var(--text-muted) 28%, transparent);
	}
	.btn--outline-muted:hover:not(:disabled) {
		background-color: var(--bg-hover);
		color: var(--text-primary);
	}

	/* Colored outline for actions inside infos/snacks (parent can override via .btn--outline-accent) */
	.btn--outline-accent {
		background-color: var(--bg-panel);
		color: var(--text-secondary);
		border-color: color-mix(in srgb, var(--text-muted) 35%, transparent);
	}
	.btn--outline-accent:hover:not(:disabled) {
		background-color: var(--bg-hover);
	}

	/* Filled accent (primary / call to action) */
	.btn--filled-accent {
		background-color: #2563eb;
		color: white;
		border-color: #2563eb;
	}
	.btn--filled-accent:hover:not(:disabled) {
		background-color: #3b82f6;
		border-color: #3b82f6;
	}

	/* Filled warn (destructive) */
	.btn--filled-warn {
		background-color: #dc2626;
		color: white;
		border-color: #dc2626;
	}
	.btn--filled-warn:hover:not(:disabled) {
		background-color: #ef4444;
		border-color: #ef4444;
	}

	.btn__icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		width: 0.875rem;
		height: 0.875rem;
	}
	.btn__icon :global(svg) {
		width: 100%;
		height: 100%;
	}
	/* Icon-only: smaller icon relative to button for clearer padding */
	.btn--icon-only .btn__icon {
		width: 0.75rem;
		height: 0.75rem;
	}
	.btn__label {
		white-space: nowrap;
	}
	.btn__spinner {
		display: inline-block;
		width: 0.875rem;
		height: 0.875rem;
		border: 2px solid currentColor;
		border-right-color: transparent;
		border-radius: 50%;
		animation: btn-spin 0.6s linear infinite;
	}
	@keyframes btn-spin {
		to {
			transform: rotate(360deg);
		}
	}
</style>
