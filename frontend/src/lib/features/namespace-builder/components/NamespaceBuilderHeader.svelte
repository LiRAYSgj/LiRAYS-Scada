<script lang="ts">
	import { Button } from "$lib/components/Button";
	import { Input } from "$lib/components/ui/input";
	import type { EditorMode } from "../types.js";

	let {
		mode,
		disabled = false,
		onOpenVisualTree,
		onOpenYaml,
		onAddRoot,
		onImportYamlClick,
		onFormatYaml,
		importInput = $bindable<HTMLInputElement | null>(null),
		onImportFileChange
	}: {
		mode: EditorMode;
		disabled?: boolean;
		onOpenVisualTree: () => void;
		onOpenYaml: () => void;
		onAddRoot: () => void;
		onImportYamlClick: () => void;
		onFormatYaml: () => void;
		importInput?: HTMLInputElement | null;
		onImportFileChange: (e: Event) => void;
	} = $props();
</script>

<header
	class="flex shrink-0 items-center justify-between gap-2 border-b border-border pb-2"
>
	<div class="flex gap-1.5">
		<Button
			variant="outline-muted"
			label="Visual Tree"
			title="Visual Tree"
			selected={mode === "visual-tree"}
			disabled={disabled}
			onclick={onOpenVisualTree}
		/>
		<Button
			variant="outline-muted"
			label="YAML"
			title="YAML"
			selected={mode === "code-yaml"}
			disabled={disabled}
			onclick={onOpenYaml}
		/>
	</div>
	<div class="flex gap-1.5">
		{#if mode === "visual-tree"}
			<Button
				variant="outline-muted"
				label="Add root node"
				title="Add root node"
				disabled={disabled}
				onclick={onAddRoot}
			/>
		{/if}
		{#if mode === "code-yaml"}
			<Button
				variant="outline-muted"
				label="Import YML"
				title="Import YML"
				disabled={disabled}
				onclick={onImportYamlClick}
			/>
			<Button
				variant="outline-muted"
				label="Format code"
				title="Format code"
				disabled={disabled}
				onclick={onFormatYaml}
			/>
		{/if}
	</div>
	<Input
		id="namespace-builder-import-yaml"
		name="import-yaml"
		bind:ref={importInput}
		type="file"
		accept=".yml,.yaml,text/yaml,text/plain"
		class="hidden"
		onchange={onImportFileChange}
	/>
</header>
