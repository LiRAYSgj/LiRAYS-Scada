<script lang="ts">
	import BaseAssetShell from './BaseAssetShell.svelte';
	import { toPercent } from '../controller';
	import type { PlantAssetComponentProps } from '../types';

	let { data, selected = false }: PlantAssetComponentProps = $props();
	const speedPercent = $derived(toPercent(data.liveValue));
	const durationSec = $derived(Number((4 - (speedPercent / 100) * 3.6).toFixed(2)));
</script>

<BaseAssetShell {data} {selected} assetClass="w-[170px] h-[220px]">
	{#snippet body()}
		<svg viewBox="0 0 100 120" class="h-[120px] w-full">
			<circle
				cx="50"
				cy="58"
				r="34"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				opacity="0.4"
			/>
			<g
				class="fan-blades"
				style={`transform-origin: 50px 58px; animation-duration: ${durationSec}s; animation-play-state: ${speedPercent > 0 ? 'running' : 'paused'};`}
			>
				<ellipse cx="50" cy="33" rx="8" ry="20" fill="currentColor" opacity="0.85" />
				<ellipse
					cx="72"
					cy="68"
					rx="8"
					ry="20"
					fill="currentColor"
					opacity="0.85"
					transform="rotate(120 72 68)"
				/>
				<ellipse
					cx="28"
					cy="68"
					rx="8"
					ry="20"
					fill="currentColor"
					opacity="0.85"
					transform="rotate(-120 28 68)"
				/>
			</g>
			<circle cx="50" cy="58" r="7" fill="currentColor" />
			<rect x="46" y="88" width="8" height="20" rx="3" fill="currentColor" opacity="0.7" />
		</svg>
	{/snippet}
	{#snippet footer()}
		<p>Speed: {speedPercent}%</p>
	{/snippet}
</BaseAssetShell>

<style>
	.fan-blades {
		animation-name: fan-spin;
		animation-timing-function: linear;
		animation-iteration-count: infinite;
	}

	@keyframes fan-spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}
</style>
