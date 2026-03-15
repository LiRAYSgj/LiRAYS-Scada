<script lang="ts">
	import { onMount } from 'svelte';
	import { Button } from '$lib/components/Button';
	import { ChevronRight, LoaderCircle } from 'lucide-svelte';
	import {
		resolveMenuOptions,
		type MenuContext,
		type MenuOption,
		type MenuOptionsResolver
	} from '../context-menu';

	interface MenuLayer {
		x: number;
		y: number;
		items: MenuOption[];
		loading: boolean;
	}

	interface Props {
		anchorX: number;
		anchorY: number;
		context: MenuContext;
		rootResolver: MenuOptionsResolver;
		onClose: () => void;
	}

	const LAYER_WIDTH = 220;
	const LAYER_GAP = 6;

	let { anchorX, anchorY, context, rootResolver, onClose }: Props = $props();
	let menuLayers = $state<MenuLayer[]>([
		{
			x: 0,
			y: 0,
			items: [],
			loading: true
		}
	]);

	async function loadRootMenu(): Promise<void> {
		const items = await resolveMenuOptions(rootResolver(context));
		menuLayers = [
			{
				x: anchorX,
				y: anchorY,
				items,
				loading: false
			}
		];
	}

	function closeMenus(): void {
		onClose();
	}

	async function openSubmenu(layerIndex: number, item: MenuOption): Promise<void> {
		menuLayers = menuLayers.slice(0, layerIndex + 1);

		if (item.separator || (!item.children && !item.getChildren)) {
			return;
		}

		const parentLayer = menuLayers[layerIndex];
		const baseX = parentLayer.x + LAYER_WIDTH + LAYER_GAP;
		const submenuLayer: MenuLayer = {
			x: baseX,
			y: parentLayer.y,
			items: [],
			loading: true
		};
		menuLayers = [...menuLayers, submenuLayer];

		const items = item.children
			? item.children
			: await resolveMenuOptions(item.getChildren ? item.getChildren(context) : []);

		menuLayers = [
			...menuLayers.slice(0, layerIndex + 1),
			{
				x: baseX,
				y: parentLayer.y,
				items,
				loading: false
			}
		];
	}

	async function selectOption(item: MenuOption): Promise<void> {
		if (item.separator || item.disabled) {
			return;
		}

		if (item.children || item.getChildren) {
			return;
		}

		if (item.onSelect) {
			await item.onSelect(context);
		}
		closeMenus();
	}

	onMount(() => {
		void loadRootMenu();

		const onPointerDown = (event: PointerEvent) => {
			const target = event.target as HTMLElement | null;
			if (!target?.closest('[data-context-menu]')) {
				closeMenus();
			}
		};

		const onEscape = (event: KeyboardEvent) => {
			if (event.key === 'Escape') {
				closeMenus();
			}
		};

		window.addEventListener('pointerdown', onPointerDown);
		window.addEventListener('keydown', onEscape);
		return () => {
			window.removeEventListener('pointerdown', onPointerDown);
			window.removeEventListener('keydown', onEscape);
		};
	});
</script>

{#each menuLayers as layer, layerIndex}
	<div
		data-context-menu
		class="fixed z-50 w-[220px] rounded-md border border-black/15 bg-(--bg-panel) p-1 shadow-lg dark:border-white/10"
		style={`background-color: var(--bg-panel); left:${layer.x}px;top:${layer.y}px;`}
	>
		{#if layer.loading}
			<div class="flex items-center gap-2 px-2 py-1.5 text-xs text-slate-500 dark:text-slate-300">
				<LoaderCircle class="h-3.5 w-3.5 animate-spin" />
				<span>Loading...</span>
			</div>
		{:else}
			{#each layer.items as item (item.id)}
				{#if item.separator}
					<div
						class="my-1 border-t border-black/10 dark:border-white/10"
						role="separator"
						aria-hidden="true"
					></div>
				{:else}
					<Button
						variant="ghost"
						disabled={item.disabled}
						class="w-full justify-between rounded px-2 py-1.5 text-left text-xs text-slate-700 dark:text-slate-200"
						onmouseenter={() => {
							void openSubmenu(layerIndex, item);
						}}
						onclick={() => {
							void selectOption(item);
						}}
					>
						{#snippet children()}
							<span>{item.label}</span>
							{#if item.children || item.getChildren}
								<ChevronRight class="h-3.5 w-3.5" />
							{/if}
						{/snippet}
					</Button>
				{/if}
			{/each}
		{/if}
	</div>
{/each}
