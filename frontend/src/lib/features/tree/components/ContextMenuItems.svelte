<script lang="ts">
	import ContextMenuItems from "./ContextMenuItems.svelte";
	import * as ContextMenu from "$lib/components/ui/context-menu";
	import type { MenuContext } from "../context-menu";
	import type { ResolvedMenuOption } from "./context-menu-types";

	interface Props {
		items: ResolvedMenuOption[];
		context: MenuContext;
		onLeafSelect: () => void;
	}

	let { items, context, onLeafSelect }: Props = $props();

	async function handleLeafSelect(
		event: Event,
		item: ResolvedMenuOption,
	): Promise<void> {
		event.preventDefault();
		if (item.disabled || item.separator || item.children?.length) return;
		await item.onSelect?.(context);
		onLeafSelect();
	}
</script>

{#each items as item (item.id)}
	{#if item.separator}
		<ContextMenu.Separator />
	{:else if item.children && item.children.length > 0}
		<ContextMenu.Sub>
			<ContextMenu.SubTrigger class="gap-2">
				{#if item.icon}
					{@const Icon = item.icon}
					<Icon class="size-3.5" aria-hidden="true" />
				{/if}
				<span class="truncate">{item.label}</span>
			</ContextMenu.SubTrigger>
			<ContextMenu.SubContent class="w-[220px]">
				<ContextMenuItems
					items={item.children}
					{context}
					{onLeafSelect}
				/>
			</ContextMenu.SubContent>
		</ContextMenu.Sub>
	{:else}
		<ContextMenu.Item
			disabled={item.disabled}
			class="gap-2"
			onSelect={(event) => {
				void handleLeafSelect(event, item);
			}}
		>
			{#if item.icon}
				{@const Icon = item.icon}
				<Icon class="size-3.5" aria-hidden="true" />
			{/if}
			<span class="truncate">{item.label}</span>
		</ContextMenu.Item>
	{/if}
{/each}
