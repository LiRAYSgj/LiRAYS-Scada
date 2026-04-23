<script lang="ts">
	let {
		editorHost = $bindable<HTMLDivElement | null>(null),
		monacoInitError,
		parseError
	}: {
		editorHost?: HTMLDivElement | null;
		monacoInitError: string;
		parseError: string;
	} = $props();
</script>

<div class="flex min-h-0 flex-1 flex-col overflow-hidden">
	<div class="flex h-full min-h-0 flex-col">
		<div bind:this={editorHost} class="h-full min-h-[520px]"></div>
		{#if monacoInitError}
			<div
				class="border-t border-red-700 px-2 py-1.5 text-xs text-red-200"
				style="background: color-mix(in srgb, #7f1d1d 50%, transparent)"
			>
				Monaco init error: {monacoInitError}
			</div>
		{/if}
		{#if parseError}
			<div
				class="border-t border-red-700 px-2 py-1.5 text-xs text-red-200"
				style="background: color-mix(in srgb, #7f1d1d 50%, transparent)"
			>
				{parseError}
			</div>
		{/if}
	</div>
</div>
