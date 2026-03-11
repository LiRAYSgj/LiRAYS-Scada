<script lang="ts">
	import { onDestroy } from 'svelte';
	import BaseAssetShell from './BaseAssetShell.svelte';
	import type { TagScalarValue } from '$lib/core/ws/types';
	import type { PlantAssetComponentProps } from '../types';

	type InputKind = 'text' | 'number';

	let { data, selected = false }: PlantAssetComponentProps = $props();

	const dataType = $derived((data.sourceNode.dataType ?? '').toLowerCase());
	const inputType = $derived<InputKind>(dataType === 'integer' || dataType === 'float' ? 'number' : 'text');
	const step = $derived(dataType === 'integer' ? '1' : dataType === 'float' ? '0.01' : undefined);
	const min = $derived(inputType === 'number' ? '0' : undefined);
	let value = $state('');
	let isEditing = $state(false);
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;

	onDestroy(() => {
		if (debounceTimer) {
			clearTimeout(debounceTimer);
		}
	});

	$effect(() => {
		if (!isEditing && data.liveValue !== undefined) {
			value = String(data.liveValue);
		}
	});

	function parseTypedValue(raw: string): TagScalarValue {
		if (inputType === 'text') {
			return raw;
		}
		const numeric = Number(raw);
		if (!Number.isFinite(numeric)) {
			return 0;
		}
		return dataType === 'integer' ? Math.round(numeric) : numeric;
	}

	function handleInput(event: Event): void {
		const target = event.currentTarget as HTMLInputElement;
		value = target.value;
		if (debounceTimer) {
			clearTimeout(debounceTimer);
		}
		debounceTimer = setTimeout(() => {
			commitValue();
		}, 300);
	}

	function commitValue(): void {
		if (!value.length && inputType === 'number') {
			return;
		}
		data.onWriteValue?.(parseTypedValue(value));
	}

	function handleBlur(): void {
		isEditing = false;
		if (debounceTimer) {
			clearTimeout(debounceTimer);
			debounceTimer = null;
		}
		commitValue();
	}

	function handleFocus(): void {
		isEditing = true;
	}

	function handleKeydown(event: KeyboardEvent): void {
		if (event.key !== 'Enter') {
			return;
		}
		if (debounceTimer) {
			clearTimeout(debounceTimer);
			debounceTimer = null;
		}
		commitValue();
		(event.currentTarget as HTMLInputElement).blur();
	}

	function preventWheelIncrement(event: WheelEvent): void {
		if (inputType !== 'number') {
			return;
		}
		event.preventDefault();
	}
</script>

<BaseAssetShell {data} {selected} assetClass="w-[230px] h-[170px]">
	{#snippet body()}
		<div
			class="flex h-[110px] w-full flex-col justify-center gap-2 rounded border border-black/15 p-2 dark:border-white/15"
		>
			<input
				type={inputType}
				{step}
				{min}
				value={value}
				oninput={handleInput}
				onfocus={handleFocus}
				onblur={handleBlur}
				onkeydown={handleKeydown}
				onwheel={preventWheelIncrement}
				onmousedown={(event) => event.stopPropagation()}
				class="w-full rounded border border-black/15 bg-(--bg-muted) px-2 py-1.5 text-xs outline-none focus:border-sky-500 dark:border-white/15"
				placeholder="Enter value"
			/>
			<p class="text-[10px] text-(--text-muted)">
				Type: {data.sourceNode.dataType ?? 'Text'} {#if inputType === 'number'}(step {step}, min {min}){/if}
			</p>
		</div>
	{/snippet}
	{#snippet footer()}
		<p>Write input</p>
	{/snippet}
</BaseAssetShell>
