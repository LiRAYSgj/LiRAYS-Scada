<script lang="ts">
	import { onMount } from "svelte";
	import { ContextMenu as ContextMenuPrimitive } from "bits-ui";
	import * as ContextMenu from "$lib/components/ui/context-menu";
	import Loader2Icon from "@lucide/svelte/icons/loader-2";
	import ContextMenuItems from "./ContextMenuItems.svelte";
	import {
		resolveMenuOptions,
		type MenuContext,
		type MenuOption,
		type MenuOptionsResolver,
	} from "../context-menu";
	import type { ResolvedMenuOption } from "./context-menu-types";

	interface Props {
		anchorX: number;
		anchorY: number;
		context: MenuContext;
		rootResolver: MenuOptionsResolver;
		onClose: () => void;
	}

	let { anchorX, anchorY, context, rootResolver, onClose }: Props = $props();

	let open = $state(true);
	let loading = $state(true);
	let items = $state<ResolvedMenuOption[]>([]);

	const virtualAnchor = $derived.by<ContextMenuPrimitive.ContentProps["customAnchor"]>(() => ({
		getBoundingClientRect: () => new DOMRect(anchorX, anchorY, 1, 1),
	}));

	async function hydrateMenuOptions(options: MenuOption[]): Promise<ResolvedMenuOption[]> {
		const hydrated: ResolvedMenuOption[] = [];
		for (const option of options) {
			const next: ResolvedMenuOption = {
				id: option.id,
				label: option.label,
				icon: option.icon,
				separator: option.separator,
				disabled: option.disabled,
				onSelect: option.onSelect,
			};
			const childrenInput = option.children
				? option.children
				: option.getChildren
					? await resolveMenuOptions(option.getChildren(context))
					: [];
			if (childrenInput.length > 0) {
				next.children = await hydrateMenuOptions(childrenInput);
			}
			hydrated.push(next);
		}
		return hydrated;
	}

	async function loadRootMenu(): Promise<void> {
		loading = true;
		const rootItems = await resolveMenuOptions(rootResolver(context));
		items = await hydrateMenuOptions(rootItems);
		loading = false;
	}

	function closeMenu(): void {
		if (!open) {
			onClose();
			return;
		}
		open = false;
		onClose();
	}

	onMount(() => {
		void loadRootMenu();
	});
</script>

<ContextMenu.Root bind:open>
	<ContextMenu.Content
		customAnchor={virtualAnchor}
		side="right"
		align="start"
		sideOffset={2}
		class="z-50 w-[220px]"
		onEscapeKeydown={closeMenu}
		onFocusOutside={closeMenu}
		onInteractOutside={closeMenu}
	>
		{#if loading}
			<ContextMenu.Item disabled>
				<Loader2Icon class="size-3.5 animate-spin" />
				<span>Loading...</span>
			</ContextMenu.Item>
		{:else}
			<ContextMenuItems
				{items}
				{context}
				onLeafSelect={closeMenu}
			/>
		{/if}
	</ContextMenu.Content>
</ContextMenu.Root>
