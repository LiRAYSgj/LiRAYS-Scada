<script lang="ts">
	import BaseAssetShell from './BaseAssetShell.svelte';
	import { toPercent } from '../controller';
	import type { PlantAssetComponentProps } from '../types';

	let { data, selected = false }: PlantAssetComponentProps = $props();
	const tankLevel = $derived(toPercent(data.liveValue));
	const svgIdSuffix = $derived((data.symbolId ?? data.title).replace(/[^a-zA-Z0-9_-]/g, '-'));
	const tankFillId = $derived(`tank-fill-${svgIdSuffix}`);
	const tankClipId = $derived(`tank-clip-${svgIdSuffix}`);
</script>

<BaseAssetShell {data} {selected} assetClass="w-[170px] h-[280px]">
	{#snippet body()}
		<svg viewBox="0 0 154 170" class="h-[170px] w-full">
			<defs>
				<linearGradient id={tankFillId} x1="0%" y1="0%" x2="0%" y2="100%">
					<stop offset="0%" stop-color="#60a5fa" />
					<stop offset="100%" stop-color="#1d4ed8" />
				</linearGradient>
			</defs>
			<rect
				x="1"
				y="8"
				width="152"
				height="160"
				rx="18"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
			/>
			<clipPath id={tankClipId}>
				<rect x="1" y="10" width="152" height="156" rx="16" />
			</clipPath>
			<rect
				x="1"
				y={166 - (tankLevel / 100) * 156}
				width="152"
				height={(tankLevel / 100) * 156}
				fill={`url(#${tankFillId})`}
				clip-path={`url(#${tankClipId})`}
				style="transition: all 700ms ease;"
			/>
		</svg>
	{/snippet}
	{#snippet footer()}
		<p>Level: {tankLevel}%</p>
	{/snippet}
</BaseAssetShell>
