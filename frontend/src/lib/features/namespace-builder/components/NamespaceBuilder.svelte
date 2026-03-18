<script lang="ts">
	/**
	 * Namespace builder: two modes — visual tree (CRUD + DnD) and code YAML (Monaco).
	 * YAML format: "name: Type" (variable) or "name:" (folder). Leaf type auto-fill runs when
	 * strict parse fails; cursor-aware skips avoid rewriting the line the user is on.
	 */
	import { browser } from '$app/environment';
	import { onDestroy, onMount, tick } from 'svelte';
	import type { editor as MonacoEditorNamespace } from 'monaco-editor';
	import type { EditorMode, NamespaceNode } from '../types.js';
	import * as nsYaml from '../namespace-yaml.js';
	import { Button } from '$lib/components/Button';
	import NamespaceBuilderHeader from './NamespaceBuilderHeader.svelte';
	import NamespaceBuilderTreeRow from './NamespaceBuilderTreeRow.svelte';
	import NamespaceBuilderYamlPanel from './NamespaceBuilderYamlPanel.svelte';
	import {
		ROW_HEIGHT,
		OVERSCAN,
		flatten,
		findRowById,
		findParentContainer,
		findNodeLocation,
		isDescendant,
		isDropAllowed,
		resolveDropPositionForPointer,
		getGhostParent
	} from './namespace-tree-helpers.js';
	import {
		UNS_YAML_LANGUAGE_ID,
		UNS_YAML_THEME_DARK_ID,
		UNS_YAML_THEME_LIGHT_ID,
		getYamlLiteTokenizer,
		getUnsYamlDarkTheme,
		getUnsYamlLightTheme,
		getMonacoThemeId,
		getYamlCompletionProvider,
		removeEditorContextMenuItems
	} from '../monaco-yaml-config.js';

	export let initialMode: EditorMode = 'visual-tree';
	export let initialText = '';
	export let allowedDataTypes: string[] = ['Float', 'Integer', 'Text', 'Boolean'];
	/** App theme so the YAML editor matches dark/light mode. */
	export let colorMode: 'light' | 'dark' = 'dark';
	/** True while Create is in progress (disables header + tree actions, hides Cancel in parent). */
	export let createLoading = false;
	/** Called whenever YAML validity changes (so parent can disable Create button). */
	export let onValidityChange: ((valid: boolean) => void) | undefined = undefined;
	/** Called when ast or yamlText change (e.g. after parse or edit). */
	export let onChange: ((detail: { ast: NamespaceNode[]; yamlText: string }) => void) | undefined = undefined;

	let mode: EditorMode = initialMode;
	let ast: NamespaceNode[] = [];
	let yamlText = initialText || '';
	let parseError = '';

	$: yamlValid = !parseError && !monacoInitError && yamlText.trim() !== '';
	$: onValidityChange?.(yamlValid);

	let editingNodeId: string | null = null;
	let editingName = '';

	let editorHost: HTMLDivElement | null = null;
	/** Increment when opening YAML tab so {#key} gives Monaco a fresh DOM node (avoids context attribute reuse). */
	let yamlEditorMountId = 0;
	/** Prevents concurrent setupMonaco (reactive block can fire twice before editor is set → double create on same host). */
	let monacoCreating = false;

	async function openYamlTab(): Promise<void> {
		mode = 'code-yaml';
		await tick();
		// Keep Monaco mounted between tab switches — only layout; recreate only if missing
		if (editor && typeof editor.layout === 'function') {
			editor.layout();
			if (model && model.getValue() !== yamlText) {
				suppressEditorSync = true;
				model.setValue(yamlText);
				suppressEditorSync = false;
			}
		} else {
			await ensureYamlEditorReady();
		}
	}
	let monaco: any = null;
	let editor: any = null;
	let model: any = null;
	let suppressEditorSync = false;
	let parseTimer: ReturnType<typeof setTimeout> | null = null;
	let monacoInitError = '';
	let monacoCssLoaded = false;
	let monacoWorkerCtor: any = null;

	let importInput: HTMLInputElement | null = null;
	let editingInputEl: HTMLInputElement | null = null;
	let draggedNodeId: string | null = null;
	/** before = sibling above target; child = drop on bottom half → nest under target */
	let dropTarget: { rowId: string; position: 'before' | 'child'; allowed: boolean } | null =
		null;
	let dragGhostEl: HTMLElement | null = null;
	let isPointerDragging = false;

	/** Virtual scroll (same pattern as VariableTree.svelte) */
	let treeViewportEl: HTMLDivElement | null = null;
	let scrollTop = 0;
	let viewportHeight = 0;
	/** Kept for teardown; also satisfies stale HMR bundles that still call detachTreeViewportResizeObserver. */
	let treeViewportResizeObserver: ResizeObserver | null = null;

	function detachTreeViewportResizeObserver(): void {
		treeViewportResizeObserver?.disconnect();
		treeViewportResizeObserver = null;
	}

	/** Attach ResizeObserver when viewport mounts (VariableTree pattern). */
	function viewportObserver(node: HTMLDivElement): { destroy: () => void } {
		detachTreeViewportResizeObserver();
		viewportHeight = node.clientHeight;
		const ro = new ResizeObserver(() => {
			viewportHeight = node.clientHeight;
		});
		ro.observe(node);
		treeViewportResizeObserver = ro;
		return {
			destroy() {
				detachTreeViewportResizeObserver();
			}
		};
	}

	function nextId(): string {
		return `node-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
	}

	function createNode(): NamespaceNode {
		return {
			id: nextId(),
			name: '<New Node>',
			kind: 'variable',
			dataType: allowedDataTypes[0] ?? 'Float',
			rangeStart: '',
			rangeEnd: '',
			rangeStep: '',
			children: []
		};
	}

	/**
	 * Serializes ast → yamlText and syncs Monaco. Always clears parseError because
	 * serializeYamlLike only emits valid structure — visual tree is source of truth
	 * when editing from the tree (needed before virtual scroll + DnD so state stays consistent).
	 */
	function updateYamlFromAst(): void {
		yamlText = nsYaml.serializeYamlLike(ast, 0, allowedDataTypes);
		parseError = '';
		if (model) {
			const current = model.getValue();
			if (current !== yamlText) {
				suppressEditorSync = true;
				model.setValue(yamlText);
				suppressEditorSync = false;
			}
		}
		onChange?.({ ast, yamlText });
	}

	// ---------- YAML parse / serialize (see namespace-yaml.ts) ----------

	/** Parse current YAML and return nested JSON; throws if invalid. */
	export function buildNamespaceJsonFromYaml(): Record<string, unknown> {
		const roots = nsYaml.parseYamlLike(yamlText, {}, allowedDataTypes, nextId);
		return nsYaml.astToNamespaceJson(roots, allowedDataTypes);
	}

	/** Current YAML validity (no parse error, no Monaco init error, non-empty code). Used by parent to disable Create. */
	export function getValidity(): boolean {
		return !parseError && !monacoInitError && yamlText.trim() !== '';
	}

	/** Clear the tree and YAML content (e.g. when dialog is closed). */
	export function reset(): void {
		ast = [];
		yamlText = '';
		parseError = '';
		editingNodeId = null;
		editingName = '';
		if (model) {
			suppressEditorSync = true;
			model.setValue('');
			suppressEditorSync = false;
		}
		onChange?.({ ast, yamlText });
	}

	// ---------- Leaf type auto-fill (code editor) ----------
	// When strict parse fails (e.g. leaf has "Power:" without type), we try permissive parse
	// + normalize leaf folders to variables + serialize. We only replace folder-only lines
	// with "name: Type"; all other lines (including the line the user is typing) stay from before.
	// Line helpers and skip logic live below.

	/**
	 * Apply afterYaml by replacing only changed lines via executeEdits so the cursor
	 * and selection stay on the line the user moved to. Falls back to setValue if
	 * line counts differ or model is out of sync.
	 */
	function applyYamlPreservingCursor(beforeYaml: string, afterYaml: string): boolean {
		if (!editor || !model || !monaco || model.getValue() !== beforeYaml) return false;
		const beforeLines = beforeYaml.split('\n');
		let afterLines = afterYaml.split('\n');
		// Must match line count for line-by-line replace; pad with blank lines if before had blank tail only
		if (afterLines.length < beforeLines.length) {
			const padded = afterLines.slice();
			for (let i = afterLines.length; i < beforeLines.length; i += 1) {
				if (beforeLines[i].trim() !== '') return false;
				padded.push('');
			}
			afterLines = padded;
			afterYaml = afterLines.join('\n');
		}
		if (beforeLines.length !== afterLines.length) return false;
		const changed: number[] = [];
		for (let i = 0; i < beforeLines.length; i += 1) {
			if (beforeLines[i] !== afterLines[i]) changed.push(i);
		}
		if (changed.length === 0) return true;
		// Save cursor by line/column — selection objects can be invalid after edits
		const pos = editor.getPosition();
		const lineNumber = pos?.lineNumber ?? 1;
		const column = pos?.column ?? 1;
		const cursorLineIndex = lineNumber - 1;
		suppressEditorSync = true;
		try {
			// Replace only the *content* of each changed line (no trailing \n) so blank lines
			// the user added below (Enter) are not swallowed by the edit.
			// Never overwrite the line the user is typing on when it's an incomplete name (no colon).
			for (let k = changed.length - 1; k >= 0; k -= 1) {
				const i = changed[k];
				const text =
					i === cursorLineIndex && nsYaml.isPlainNameLine(beforeLines[i])
						? beforeLines[i]
						: afterLines[i];
				const line = i + 1;
				const maxCol = model.getLineMaxColumn(line);
				const range = new monaco.Range(line, 1, line, maxCol);
				editor.executeEdits('leaf-autofill', [{ range, text }]);
			}
		} finally {
			suppressEditorSync = false;
		}
		// Restore after Monaco applies edits (sync setPosition often gets overwritten)
		const restore = () => {
			try {
				const maxLine = model.getLineCount();
				const safeLine = Math.min(lineNumber, maxLine);
				const maxCol = model.getLineMaxColumn(safeLine);
				const safeCol = Math.min(column, maxCol);
				editor.setPosition({ lineNumber: safeLine, column: safeCol });
				editor.revealPositionInCenterIfOutsideViewport({
					lineNumber: safeLine,
					column: safeCol
				});
			} catch {
				/* ignore */
			}
		};
		restore();
		requestAnimationFrame(() => {
			restore();
			requestAnimationFrame(restore);
		});
		let value = model.getValue();
		if (value !== afterYaml) {
			// Edits didn’t match — full replace but always restore caret (setValue resets to 1,1)
			suppressEditorSync = true;
			model.setValue(afterYaml);
			suppressEditorSync = false;
			const restoreAfterSetValue = () => {
				try {
					const maxLine = model.getLineCount();
					const safeLine = Math.min(lineNumber, maxLine);
					const maxCol = Math.max(1, model.getLineMaxColumn(safeLine));
					const safeCol = Math.min(Math.max(1, column), maxCol);
					editor.setPosition({ lineNumber: safeLine, column: safeCol });
					editor.revealPositionInCenterIfOutsideViewport({
						lineNumber: safeLine,
						column: safeCol
					});
					editor.focus();
				} catch {
					/* ignore */
				}
			};
			restoreAfterSetValue();
			setTimeout(restoreAfterSetValue, 0);
			requestAnimationFrame(() => {
				restoreAfterSetValue();
				requestAnimationFrame(restoreAfterSetValue);
			});
		}
		value = model.getValue();
		return value === afterYaml || value.replace(/\r\n/g, '\n') === afterYaml.replace(/\r\n/g, '\n');
	}

	/**
	 * Permissive parse + inject default type on folder-only leaves + re-serialize; then apply to editor.
	 * Used when strict parse fails due to missing type. With skipWhenCursorOnModifiedLine:
	 * - Skip 1: cursor on folder-only line -> wait until user moves (click/arrows).
	 * - Skip 2: cursor on blank line under folder-only (after Enter) -> wait for next key (space/tab = no fill, other = fill via onKeyDown).
	 * - Skip 3: cursor on a folder-only line whose content would change -> avoid overwriting; other lines (e.g. typing "d") still allow filling previous.
	 */
	function tryParseAndAutoFillLeafTypes(options?: {
		skipWhenCursorOnModifiedLine?: boolean;
	}): boolean {
		try {
			// Use current editor content so we never overwrite what the user just typed (e.g. "assa")
			// with a merge built from stale yamlText (e.g. "ass" -> "ass: Float").
			const contentToUse =
				editor && model ? model.getValue() : yamlText;
			const roots = nsYaml.parseYamlLike(contentToUse, { skipLeafTypeValidation: true }, allowedDataTypes, nextId);
			nsYaml.normalizeLeafFoldersToVariables(roots, allowedDataTypes);
			nsYaml.normalizeFolderState(roots);
			let afterYaml = nsYaml.serializeYamlLike(roots, 0, allowedDataTypes);
			// Don't convert folder-only lines that are followed by a blank line (user pressed Enter
			// and is about to type a child) — keep them as folders so we never add ": Float".
			const beforeLines = contentToUse.split('\n');
			const afterLines = afterYaml.split('\n');
			if (beforeLines.length === afterLines.length) {
				for (let i = 0; i < beforeLines.length - 1; i += 1) {
					if (
						nsYaml.isFolderOnlyLine(beforeLines[i]) &&
						beforeLines[i + 1].trim() === ''
					) {
						afterLines[i] = beforeLines[i];
					}
				}
				afterYaml = afterLines.join('\n');
			}
			let cursorLineForMerge = 0;
			if (options?.skipWhenCursorOnModifiedLine && editor && model) {
				try {
					if (model.getValue() !== contentToUse) return false;
					const pos = editor.getPosition();
					if (pos) {
						cursorLineForMerge = pos.lineNumber;
						const beforeLinesForSkip = contentToUse.split('\n');
						const cursorIndex = pos.lineNumber - 1;
						if (
							cursorIndex >= 0 &&
							cursorIndex < beforeLinesForSkip.length &&
							nsYaml.isFolderOnlyLine(beforeLinesForSkip[cursorIndex])
						) {
							return false;
						}
						if (
							cursorIndex > 0 &&
							cursorIndex < beforeLinesForSkip.length &&
							beforeLinesForSkip[cursorIndex].trim() === '' &&
							nsYaml.isFolderOnlyLine(beforeLinesForSkip[cursorIndex - 1])
						) {
							return false;
						}
						const afterLines = afterYaml.split('\n');
						if (
							beforeLinesForSkip.length === afterLines.length &&
							cursorIndex >= 0 &&
							cursorIndex < beforeLinesForSkip.length &&
							nsYaml.isFolderOnlyLine(beforeLinesForSkip[cursorIndex]) &&
							beforeLinesForSkip[cursorIndex] !== afterLines[cursorIndex]
						) {
							return false;
						}
					}
				} catch {
					/* ignore position read errors */
				}
			}
			// Preserve Enter-created blank line(s) so cursor line is not lost
			if (model?.getValue() === contentToUse) {
				const lineToUse =
					cursorLineForMerge > 0
						? cursorLineForMerge
						: contentToUse.split('\n').length;
				afterYaml = nsYaml.mergeTrailingBlankLinesAfterAutoFill(contentToUse, afterYaml, lineToUse);
			}
			ast = roots;
			const merged = nsYaml.buildAutoFillMergedYaml(contentToUse, afterYaml);
			if (model && model.getValue() !== merged) {
				const before = contentToUse;
				const posBefore = editor?.getPosition();
				const lineNum = posBefore?.lineNumber ?? 1;
				const colNum = posBefore?.column ?? 1;
				if (!applyYamlPreservingCursor(before, merged)) {
					suppressEditorSync = true;
					model.setValue(merged);
					suppressEditorSync = false;
					yamlText = merged;
					// applyYamlPreservingCursor failed early — still restore caret after setValue
					const fixCaret = () => {
						try {
							if (!editor || !model) return;
							const maxLine = model.getLineCount();
							const safeLine = Math.min(lineNum, maxLine);
							const maxCol = Math.max(1, model.getLineMaxColumn(safeLine));
							const safeCol = Math.min(Math.max(1, colNum), maxCol);
							editor.setPosition({ lineNumber: safeLine, column: safeCol });
							editor.revealPositionInCenterIfOutsideViewport({
								lineNumber: safeLine,
								column: safeCol
							});
							editor.focus();
						} catch {
							/* ignore */
						}
					};
					fixCaret();
					setTimeout(fixCaret, 0);
					requestAnimationFrame(() => {
						fixCaret();
						requestAnimationFrame(fixCaret);
					});
				} else {
					yamlText = model.getValue();
				}
			} else {
				yamlText = merged;
			}
			// Re-validate merged content so validation message shows regardless of cursor (e.g. "bbbb" invalid)
			const contentToValidate = model ? model.getValue() : merged;
			try {
				nsYaml.parseYamlLike(contentToValidate, {}, allowedDataTypes, nextId);
				parseError = '';
			} catch (err) {
				parseError = err instanceof Error ? err.message : 'Invalid YAML';
			}
			onChange?.({ ast, yamlText });
			return true;
		} catch {
			return false;
		}
	}

	// ---------- Parse schedule ----------
	// Content change and cursor position both schedule debounced parse. When strict parse
	// fails (e.g. leaf without type), we try auto-fill (tryParseAndAutoFillLeafTypes) with
	// cursor-aware skips before showing an error.

	function parseAndApplyYaml(): void {
		try {
			const next = nsYaml.parseYamlLike(yamlText, {}, allowedDataTypes, nextId);
			ast = next;
			parseError = '';
			onChange?.({ ast, yamlText });
		} catch (error) {
			if (tryParseAndAutoFillLeafTypes({ skipWhenCursorOnModifiedLine: true })) {
				return;
			}
			parseError = error instanceof Error ? error.message : 'Invalid YAML';
		}
	}

	function scheduleParseYaml(): void {
		if (parseTimer) clearTimeout(parseTimer);
		parseTimer = setTimeout(() => {
			parseAndApplyYaml();
		}, 250);
	}

	/** Run parse (and auto-fill) immediately; used when user types non-space on blank-under-folder line. */
	function runParseYamlNow(): void {
		if (parseTimer) {
			clearTimeout(parseTimer);
			parseTimer = null;
		}
		if (editor) yamlText = editor.getValue();
		parseAndApplyYaml();
	}

	function addRootNode(): void {
		ast = [createNode(), ...ast];
		updateYamlFromAst();
	}

	function addChildNode(nodeId: string): void {
		const row = findRowById(ast, nodeId);
		if (!row) return;
		row.node.children.push(createNode());
		nsYaml.setKindFromChildren(row.node);
		ast = [...ast];
		updateYamlFromAst();
	}

	function removeNode(nodeId: string): void {
		const row = findRowById(ast, nodeId);
		if (!row) return;
		row.parentChildren.splice(row.index, 1);
		ast = [...ast];
		updateYamlFromAst();
	}

	function indentNode(nodeId: string): void {
		const rows = flatten(ast);
		const idx = rows.findIndex((row) => row.id === nodeId);
		if (idx <= 0) return;
		const row = rows[idx];
		const prevRow = rows[idx - 1];
		row.parentChildren.splice(row.index, 1);
		prevRow.node.children.push(row.node);
		nsYaml.setKindFromChildren(prevRow.node);
		ast = [...ast];
		updateYamlFromAst();
	}

	function outdentNode(nodeId: string): void {
		const row = findRowById(ast, nodeId);
		if (!row || !row.parentId) return;
		const parentRow = findRowById(ast, row.parentId);
		if (!parentRow) return;
		row.parentChildren.splice(row.index, 1);
		const parentContainer = findParentContainer(ast, parentRow.parentId);
		const parentIndex = parentContainer.findIndex((item) => item.id === parentRow.id);
		parentContainer.splice(parentIndex + 1, 0, row.node);
		ast = [...ast];
		updateYamlFromAst();
	}

	function startEditName(nodeId: string): void {
		const row = findRowById(ast, nodeId);
		if (!row) return;
		editingNodeId = nodeId;
		editingName = row.node.name;
		setTimeout(() => {
			editingInputEl?.focus();
			editingInputEl?.select();
		}, 0);
	}

	function commitEditName(nodeId: string): void {
		// Enter commits then blur fires — ignore second call so name isn't reset to '<New Node>'
		if (editingNodeId !== nodeId) return;
		const row = findRowById(ast, nodeId);
		if (!row) return;
		row.node.name = editingName.trim() || '<New Node>';
		editingNodeId = null;
		editingName = '';
		ast = [...ast];
		updateYamlFromAst();
	}

	function updateNodeRange(nodeId: string, key: 'rangeStart' | 'rangeEnd' | 'rangeStep', value: string): void {
		const row = findRowById(ast, nodeId);
		if (!row) return;
		row.node[key] = value;
		ast = [...ast];
		updateYamlFromAst();
	}

	function updateNodeType(nodeId: string, value: string): void {
		const row = findRowById(ast, nodeId);
		if (!row || row.node.children.length > 0) return;
		row.node.kind = 'variable';
		row.node.dataType = value;
		ast = [...ast];
		updateYamlFromAst();
	}

	function buildDragGhost(rowId: string): HTMLElement | null {
		const source = document.querySelector(`[data-node-row-id="${rowId}"]`) as HTMLElement | null;
		const parent = getGhostParent(source);
		if (!source) {
			const row = findRowById(ast, rowId);
			if (!row) return null;
			const ghost = document.createElement('div');
			ghost.className =
				'fixed z-[2147483647] cursor-default whitespace-nowrap rounded-md border border-blue-600 bg-[var(--bg-panel)] px-2.5 py-1.5 text-xs text-[var(--text-primary)] opacity-85 shadow-lg pointer-events-none';
			ghost.textContent = row.node.name || '<New Node>';
			parent.appendChild(ghost);
			return ghost;
		}
		const clone = source.cloneNode(true) as HTMLElement;
		clone.classList.add('drag-ghost-clone');
		// Ghost is in DOM with source row — remove all ids first so nothing duplicates the source row.
		clone.querySelectorAll('[id]').forEach((el) => el.removeAttribute('id'));
		clone.querySelectorAll('[name]').forEach((el) => el.removeAttribute('name'));
		// Ghost form controls must still have id or name (audit) — assign unique, non-colliding ids
		const ghostUid = `drag-ghost-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`;
		clone.querySelectorAll('input, select, textarea').forEach((el, i) => {
			const id = `${ghostUid}-f${i}`;
			el.setAttribute('id', id);
			el.setAttribute('name', id);
			if (el instanceof HTMLInputElement) el.disabled = true;
			else if (el instanceof HTMLSelectElement) el.disabled = true;
			else if (el instanceof HTMLTextAreaElement) el.disabled = true;
		});
		// Preserve original row dimensions so flex layout doesn't collapse when detached
		const rect = source.getBoundingClientRect();
		if (rect.width > 0) {
			clone.style.setProperty('width', `${Math.ceil(rect.width)}px`, 'important');
			clone.style.setProperty('min-height', `${Math.ceil(rect.height)}px`, 'important');
			clone.style.setProperty('box-sizing', 'border-box', 'important');
		}
		parent.appendChild(clone);
		return clone;
	}

	function positionDragGhost(clientX: number, clientY: number): void {
		if (!dragGhostEl) return;
		// Clone is full-width; offset so cursor isn't covered.
		// Use setProperty(..., 'important') so we win over any cloned inline styles.
		const w = dragGhostEl.offsetWidth || 320;
		const left = clientX + 12;
		const maxLeft = typeof window !== 'undefined' ? window.innerWidth - w - 8 : left;
		const top = clientY + 12;
		dragGhostEl.style.setProperty('left', `${Math.min(left, maxLeft)}px`, 'important');
		dragGhostEl.style.setProperty('top', `${top}px`, 'important');
	}

	function startPointerDrag(event: PointerEvent, rowId: string): void {
		if (createLoading) return;
		event.preventDefault();
		event.stopPropagation();
		draggedNodeId = rowId;
		isPointerDragging = true;
		dropTarget = null;
		dragGhostEl = buildDragGhost(rowId);
		positionDragGhost(event.clientX, event.clientY);
		window.addEventListener('pointermove', handlePointerDragMove);
		window.addEventListener('pointerup', handlePointerDragEnd, { once: true });
		window.addEventListener('pointercancel', handlePointerDragEnd, { once: true });
	}

	function handlePointerDragMove(event: PointerEvent): void {
		if (!isPointerDragging || !draggedNodeId) return;
		positionDragGhost(event.clientX, event.clientY);
		const element = document.elementFromPoint(event.clientX, event.clientY) as HTMLElement | null;
		if (!element) {
			dropTarget = null;
			return;
		}

		const rowEl = element.closest('[data-node-row-id]') as HTMLElement | null;
		if (!rowEl) {
			dropTarget = null;
			return;
		}
		const rowId = rowEl.dataset.nodeRowId;
		if (!rowId || rowId === draggedNodeId) {
			dropTarget = null;
			return;
		}
		const position = resolveDropPositionForPointer(rowEl, event.clientY);
		const allowed = isDropAllowed(ast, draggedNodeId, rowId);
		dropTarget = { rowId, position, allowed };
	}

	function handlePointerDragEnd(): void {
		if (!isPointerDragging || !draggedNodeId) {
			clearDragState();
			return;
		}
		if (dropTarget?.allowed) {
			moveNode(draggedNodeId, dropTarget.rowId, dropTarget.position);
		}
		clearDragState();
	}

	function clearDragState(): void {
		draggedNodeId = null;
		dropTarget = null;
		isPointerDragging = false;
		if (browser) {
			window.removeEventListener('pointermove', handlePointerDragMove);
			window.removeEventListener('pointerup', handlePointerDragEnd);
			window.removeEventListener('pointercancel', handlePointerDragEnd);
		}
		if (dragGhostEl) {
			dragGhostEl.remove();
			dragGhostEl = null;
		}
	}

	function moveNode(draggedId: string, targetRowId: string, position: 'before' | 'child'): void {
		if (draggedId === targetRowId) return;

		const draggedLoc = findNodeLocation(ast, draggedId);
		const targetLoc = findNodeLocation(ast, targetRowId);
		if (!draggedLoc || !targetLoc) return;
		if (isDescendant(draggedLoc.node, targetLoc.node.id)) return;

		draggedLoc.parentChildren.splice(draggedLoc.index, 1);

		if (position === 'child') {
			targetLoc.node.children = [...targetLoc.node.children, draggedLoc.node];
			nsYaml.setKindFromChildren(targetLoc.node);
			ast = [...ast];
			updateYamlFromAst();
			return;
		}

		// before: insert as sibling before target in same parent list
		const targetContainer = targetLoc.parentChildren;
		let insertIndex = targetLoc.index;
		if (targetContainer === draggedLoc.parentChildren && draggedLoc.index < insertIndex) {
			insertIndex -= 1;
		}
		targetContainer.splice(insertIndex, 0, draggedLoc.node);
		ast = [...ast];
		updateYamlFromAst();
	}

	function importYamlClick(): void {
		importInput?.click();
	}

	function handleImportYaml(event: Event): void {
		const target = event.currentTarget as HTMLInputElement;
		const file = target.files?.[0];
		if (!file) return;
		const reader = new FileReader();
		reader.onload = () => {
			yamlText = String(reader.result ?? '');
			if (model) {
				suppressEditorSync = true;
				model.setValue(yamlText);
				suppressEditorSync = false;
			}
			parseAndApplyYaml();
		};
		reader.readAsText(file);
		target.value = '';
	}

	function formatYaml(): void {
		try {
			const parsed = nsYaml.parseYamlLike(yamlText, {}, allowedDataTypes, nextId);
			ast = parsed;
			parseError = '';
			updateYamlFromAst();
		} catch (e1) {
			// Format always applies (cursor position less important)
			if (!tryParseAndAutoFillLeafTypes({ skipWhenCursorOnModifiedLine: false })) {
				parseError = e1 instanceof Error ? e1.message : 'Invalid YAML';
			}
		}
	}

	function disposeYamlEditor(): void {
		if (!browser) return;
		try {
			editor?.dispose();
		} catch {
			/* ignore */
		}
		editor = null;
		try {
			model?.dispose();
		} catch {
			/* ignore */
		}
		model = null;
		// Host may be detached; if still in DOM, strip Monaco context so recreate doesn't throw
		if (editorHost && editorHost.isConnected) {
			editorHost.replaceChildren();
			editorHost.removeAttribute('data-keybinding-context');
			editorHost.classList.remove('monaco-editor');
		}
	}

	async function setupMonaco(): Promise<void> {
		if (!browser || !editorHost || editor || monacoCreating) return;
		monacoCreating = true;
		const host = editorHost;
		try {
			// Load Monaco heavy assets only when the YAML tab is actually opened.
			if (!monacoCssLoaded) {
				await import('monaco-editor/min/vs/editor/editor.main.css');
				monacoCssLoaded = true;
			}
			if (!monacoWorkerCtor) {
				const workerMod: any = await import(
					'monaco-editor/esm/vs/editor/editor.worker?worker'
				);
				monacoWorkerCtor = workerMod.default ?? workerMod;
			}
			(globalThis as any).MonacoEnvironment = {
				getWorker() {
					return new monacoWorkerCtor();
				}
			};
			monaco = await import('monaco-editor');
			await removeEditorContextMenuItems(monaco);
			if (editorHost !== host || editor) return;
			monaco.languages.register({ id: UNS_YAML_LANGUAGE_ID });
			monaco.languages.setMonarchTokensProvider(UNS_YAML_LANGUAGE_ID, getYamlLiteTokenizer());
			monaco.editor.defineTheme(UNS_YAML_THEME_DARK_ID, getUnsYamlDarkTheme());
			monaco.editor.defineTheme(UNS_YAML_THEME_LIGHT_ID, getUnsYamlLightTheme());
			monaco.languages.registerCompletionItemProvider(
				UNS_YAML_LANGUAGE_ID,
				getYamlCompletionProvider(monaco, allowedDataTypes)
			);
			if (editorHost !== host || editor) return;
			model = monaco.editor.createModel(yamlText, UNS_YAML_LANGUAGE_ID);
			if (editorHost !== host) {
				model.dispose();
				model = null;
				return;
			}
			editor = monaco.editor.create(host, {
				model,
				automaticLayout: true,
				minimap: { enabled: false },
				wordWrap: 'on',
				tabSize: 2,
				insertSpaces: true,
				scrollBeyondLastLine: false,
				theme: getMonacoThemeId(colorMode)
			});
			// Cut with ⌘X so shortcut shows in context menu (browser default has no keybinding in menu)
			editor.addAction({
				id: 'yaml-editor-cut',
				label: 'Cut',
				keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyX],
				contextMenuGroupId: '9_cutcopypaste',
				run: (ed: MonacoEditorNamespace.ICodeEditor) => {
					ed.trigger('keyboard', 'editor.action.clipboardCutAction', null);
				}
			});
			// Copy with ⌘C so shortcut shows in context menu
			editor.addAction({
				id: 'yaml-editor-copy',
				label: 'Copy',
				keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyC],
				contextMenuGroupId: '9_cutcopypaste',
				run: (ed: MonacoEditorNamespace.ICodeEditor) => {
					ed.trigger('keyboard', 'editor.action.clipboardCopyAction', null);
				}
			});
			// Paste via Clipboard API so context-menu Paste and Ctrl+V work (default often blocked)
			editor.addAction({
				id: 'yaml-editor-paste',
				label: 'Paste',
				keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyV],
				contextMenuGroupId: '9_cutcopypaste',
				run: (ed: MonacoEditorNamespace.ICodeEditor) => {
					if (!navigator.clipboard?.readText) return;
					navigator.clipboard.readText().then((text) => {
						const selection = ed.getSelection();
						const range = selection ?? {
							startLineNumber: ed.getPosition()?.lineNumber ?? 1,
							startColumn: ed.getPosition()?.column ?? 1,
							endLineNumber: ed.getPosition()?.lineNumber ?? 1,
							endColumn: ed.getPosition()?.column ?? 1
						};
						ed.executeEdits('paste', [{ range, text }]);
					}).catch(() => {});
				}
			});
			// After our editor's context menu opens, hide Command Palette and the broken second Paste (keep the first = ours with ⌘V)
			function pruneContextMenu(): void {
				// Find any visible context-menu-like container (Monaco often uses different class names)
				const menus = document.querySelectorAll('[role="menu"], .monaco-menu, [class*="monaco-menu"], [class*="contextview"]');
				const pasteRows: HTMLElement[] = [];
				menus.forEach((menu) => {
					// Only consider menus that are likely visible (have position or are in view)
					const menuEl = menu as HTMLElement;
					const rect = menuEl.getBoundingClientRect();
					if (rect.width < 10 || rect.height < 10) return;
					// Walk all descendants that might be a menu row (has click handler / is focusable / contains label text)
					menu.querySelectorAll('[role="menuitem"], .action-item, [class*="action-item"], [class*="menuitem"]').forEach((item) => {
						const el = item as HTMLElement;
						const raw = (el.textContent ?? '').trim();
						if (raw === 'Command Palette' || raw.startsWith('Command Palette')) {
							el.style.setProperty('display', 'none', 'important');
						}
						const firstWord = raw.split(/\s+/)[0];
						if (firstWord === 'Paste') pasteRows.push(el);
					});
				});
				// Hide the second Paste row (index 1); keep the first (ours with ⌘V)
				if (pasteRows.length >= 2) pasteRows[1].style.setProperty('display', 'none', 'important');
			}
			host.addEventListener('contextmenu', () => {
				[60, 120, 200, 350].forEach((ms) => setTimeout(pruneContextMenu, ms));
			});
			// Parse triggers: content change and cursor position both schedule debounced parse,
			// so auto-fill can run after the user leaves a folder-only line (click/arrows).
			editor.onDidChangeModelContent(() => {
				if (suppressEditorSync) return;
				yamlText = editor.getValue();
				scheduleParseYaml();
			});
			editor.onDidChangeCursorPosition(() => {
				if (suppressEditorSync) return;
				yamlText = editor.getValue();
				scheduleParseYaml();
			});
			// After Enter, cursor is on blank line under folder-only. Next key: space/tab -> no fill (folder+child); any other -> run parse to fill previous line.
			editor.onKeyDown((e: { browserEvent: KeyboardEvent }) => {
				const key = e.browserEvent?.key;
				if (key === ' ' || key === 'Tab') return;
				const pos = editor.getPosition();
				if (!pos || !model) return;
				const lineContent = model.getLineContent(pos.lineNumber);
				const prevLine =
					pos.lineNumber > 1 ? model.getLineContent(pos.lineNumber - 1) : '';
				if (lineContent.trim() === '' && nsYaml.isFolderOnlyLine(prevLine)) {
					setTimeout(runParseYamlNow, 0);
				}
			});
			// Monaco injects textarea(s) without name/id — DevTools Issues flags them; satisfy audit once DOM is ready
			const patchMonacoFormFields = (container: HTMLElement) => {
				container.querySelectorAll('textarea').forEach((el, i) => {
					if (!el.getAttribute('name')) el.setAttribute('name', 'monaco-yaml-textarea');
					if (!el.id) el.id = `monaco-yaml-textarea-${yamlEditorMountId}-${i}`;
				});
				container.querySelectorAll('input').forEach((el, i) => {
					if (el.type === 'hidden') return;
					if (!el.getAttribute('name')) el.setAttribute('name', 'monaco-yaml-input');
					if (!el.id) el.id = `monaco-yaml-input-${yamlEditorMountId}-${i}`;
				});
			};
			requestAnimationFrame(() => patchMonacoFormFields(host));
			monacoInitError = '';
		} catch (error) {
			monacoInitError = error instanceof Error ? error.message : 'Failed to initialize Monaco';
		} finally {
			monacoCreating = false;
		}
	}

	async function ensureYamlEditorReady(): Promise<void> {
		if (mode !== 'code-yaml') return;
		await tick();
		// Tab switch unmounts host; editor still references dead instance — always dispose if host mismatch
		if (
			editor &&
			editorHost &&
			typeof editor.getContainerDomNode === 'function' &&
			editor.getContainerDomNode() !== editorHost
		) {
			disposeYamlEditor();
		}
		if (!editor && !monacoCreating && editorHost) {
			await setupMonaco();
		}
		if (editor && model) {
			const current = model.getValue();
			if (current !== yamlText) {
				suppressEditorSync = true;
				model.setValue(yamlText);
				suppressEditorSync = false;
			}
			editor.layout();
		}
	}

	// Keep Monaco theme in sync with app color mode (e.g. after user toggles dark/light).
	$: if (monaco && editor && typeof monaco.editor.setTheme === 'function') {
		monaco.editor.setTheme(getMonacoThemeId(colorMode));
	}

	// Readonly only while waiting for Create response; editable again as soon as response arrives (success or error).
	$: if (editor && typeof editor.updateOptions === 'function') {
		editor.updateOptions({ readOnly: !!createLoading });
	}

	$: rows = flatten(ast);
	$: totalRows = rows.length;
	// While dragging, render full list so elementFromPoint can hit any row's data-node-row-id
	$: startIndex =
		isPointerDragging || totalRows === 0
			? 0
			: Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN);
	$: visibleCount = Math.max(1, Math.ceil(viewportHeight / ROW_HEIGHT) + OVERSCAN * 2);
	$: endIndex =
		isPointerDragging || totalRows === 0
			? totalRows
			: Math.min(totalRows, startIndex + visibleCount);
	$: windowRows = rows.slice(startIndex, endIndex);
	$: topPadding = isPointerDragging || totalRows === 0 ? 0 : startIndex * ROW_HEIGHT;
	$: bottomPadding =
		isPointerDragging || totalRows === 0 ? 0 : Math.max(0, (totalRows - endIndex) * ROW_HEIGHT);

	function onTreeViewportScroll(): void {
		if (treeViewportEl) scrollTop = treeViewportEl.scrollTop;
	}

	async function openVisualTreeTab(): Promise<void> {
		mode = 'visual-tree';
		scrollTop = 0;
		await tick();
		if (treeViewportEl) {
			treeViewportEl.scrollTop = 0;
		}
	}

	// Do NOT call ensureYamlEditorReady from a reactive block — it runs too often and races async setupMonaco.

	onMount(async () => {
		if (yamlText.trim()) {
			parseAndApplyYaml();
		}
		// First open on YAML tab needs a stable host; if initialMode is yaml, mountId 0 is enough
		if (initialMode === 'code-yaml') {
			await tick();
			await ensureYamlEditorReady();
		}
	});

	// When on YAML tab, ensure Monaco is ready (replaces deprecated afterUpdate)
	$: if (mode === 'code-yaml') {
		void ensureYamlEditorReady();
	}

	onDestroy(() => {
		if (parseTimer) clearTimeout(parseTimer);
		clearDragState();
		detachTreeViewportResizeObserver();
		disposeYamlEditor();
	});
</script>

<div class="flex min-h-0 flex-col gap-2 h-full">
	<NamespaceBuilderHeader
		{mode}
		disabled={createLoading}
		onOpenVisualTree={() => void openVisualTreeTab()}
		onOpenYaml={() => void openYamlTab()}
		onAddRoot={addRootNode}
		onImportYamlClick={importYamlClick}
		onFormatYaml={formatYaml}
		bind:importInput
		onImportFileChange={handleImportYaml}
	/>

	<section
		class="flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg border"
		style="border-color: color-mix(in srgb, var(--text-muted) 25%, transparent)"
	>
		<!-- Tree + YAML both stay mounted; visibility toggled so Monaco isn't recreated (highlight stays instant). -->
		<div
			class="flex min-h-0 flex-1 flex-col overflow-hidden"
			class:min-h-0={mode === 'visual-tree' && rows.length > 0}
			class:flex-1={mode === 'visual-tree'}
			hidden={mode !== 'visual-tree'}
		>
			{#if parseError && yamlText.trim() !== ''}
				<div
					class="mb-2 flex flex-wrap items-center gap-x-3 gap-y-2 rounded-md border border-red-700 px-2 py-1.5 text-xs text-red-200"
					style="background: color-mix(in srgb, #7f1d1d 50%, transparent)"
					role="alert"
				>
					<strong>YAML invalid</strong> — tree may be stale until fixed.
					<span class="min-w-0 flex-1 basis-full opacity-95">{parseError}</span>
					<Button
						variant="outline-accent"
						label="Open YAML"
						title="Open YAML"
						class="ml-auto rounded-md border border-red-200 px-2 py-0.5 text-[11px] text-red-200"
						style="background: color-mix(in srgb, #450a0a 60%, transparent)"
						onclick={openYamlTab}
					/>
				</div>
			{/if}
			<div class="w-full" class:flex-1={rows.length > 0} class:min-h-0={rows.length > 0} class:flex={rows.length > 0} class:flex-col={rows.length > 0} class:overflow-hidden={rows.length > 0}>
				{#if rows.length === 0}
					<div
						class="row-empty flex min-h-9 items-center border-b text-xs text-(--text-muted)"
						style="border-color: color-mix(in srgb, var(--text-muted) 18%, transparent)"
					>
						<div class="inline-flex items-center gap-0.5 px-1.5 opacity-100">
							<Button
								variant="icon"
								label="+"
								title="Add root node"
								disabled={createLoading}
								onclick={addRootNode}
							/>
						</div>
						<div class="relative flex w-full items-center gap-2.5 py-1 pr-2 pl-0">
							No nodes yet. Add your first root node.
						</div>
					</div>
				{:else}
					<div
						class="min-h-0 w-full flex-1 overflow-auto"
						bind:this={treeViewportEl}
						onscroll={onTreeViewportScroll}
						use:viewportObserver
					>
						<div class="pointer-events-none shrink-0" style="height: {topPadding}px" aria-hidden="true"></div>
						{#each windowRows as row (row.id)}
							<NamespaceBuilderTreeRow
								{row}
								{allowedDataTypes}
								rowHeight={ROW_HEIGHT}
								actionsDisabled={createLoading}
								{draggedNodeId}
								{dropTarget}
								{editingNodeId}
								bind:editingName
								bind:editingInputEl
								onPointerDownDrag={startPointerDrag}
								onRemove={removeNode}
								onAddChild={addChildNode}
								onIndent={indentNode}
								onOutdent={outdentNode}
								onStartEditName={startEditName}
								onCommitEditName={commitEditName}
								onUpdateRange={updateNodeRange}
								onUpdateType={updateNodeType}
							/>
						{/each}
						<div
							class="pointer-events-none shrink-0"
							style="height: {bottomPadding}px"
							aria-hidden="true"
						></div>
					</div>
				{/if}
			</div>
		</div>
		<div class="flex min-h-0 flex-1 flex-col overflow-hidden" hidden={mode !== 'code-yaml'}>
			<NamespaceBuilderYamlPanel bind:editorHost {monacoInitError} parseError={yamlText.trim() !== '' ? parseError : ''} />
		</div>
	</section>
</div>

<style>
	/* Hide Command Palette in Monaco context menus — multiple selectors (Monaco varies by version) */
	:global([data-command-id="editor.action.quickCommand"]),
	:global([data-command-id="workbench.action.showCommands"]),
	:global([aria-label="Command Palette"]),
	:global([aria-label*="Command Palette"]),
	:global([title="Command Palette"]) {
		display: none !important;
	}
	/* Parent row when the label is on a child (e.g. .action-label) */
	:global(.action-item:has([aria-label="Command Palette"])),
	:global(.action-item:has([aria-label*="Command Palette"])),
	:global([role="menuitem"]:has([aria-label="Command Palette"])),
	:global([role="menuitem"]:has([aria-label*="Command Palette"])) {
		display: none !important;
	}

	/* Cloned row ghost — !important so it wins over inline flex from source */
	:global(.drag-ghost-clone) {
		position: fixed !important;
		pointer-events: none !important;
		margin: 0 !important;
		background: var(--bg-panel) !important;
		opacity: 0.92 !important;
		filter: drop-shadow(0 10px 24px rgb(0 0 0 / 30%));
		outline: 1px solid color-mix(in srgb, #2563eb 45%, transparent);
		outline-offset: 0;
		border-radius: 0.25rem;
		z-index: 2147483647;
	}
</style>
