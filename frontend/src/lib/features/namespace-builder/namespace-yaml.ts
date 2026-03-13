/**
 * YAML-like parse/serialize and namespace AST helpers for the unified namespace builder.
 * Format: "name: Type" (variable) or "name:" (folder). Names can include range notation [start?:end:step?].
 */

import type { NamespaceNode } from './types.js';

export function lineIndent(source: string): number {
	const indentStr = source.match(/^[ \t]*/)?.[0] ?? '';
	return Math.floor(indentStr.replace(/\t/g, '  ').length / 2);
}

export function nextNonEmptyIndent(lines: string[], afterIndex: number): number | null {
	for (let i = afterIndex + 1; i < lines.length; i += 1) {
		if (lines[i].trim()) return lineIndent(lines[i]);
	}
	return null;
}

export function setKindFromChildren(node: NamespaceNode): void {
	if (node.children.length > 0) {
		node.kind = 'folder';
		node.dataType = null;
	}
}

export function composeNodeName(node: NamespaceNode): string {
	const base = node.name.trim();
	if (!node.rangeEnd.trim()) return base;
	const start = node.rangeStart.trim();
	const end = node.rangeEnd.trim();
	const step = node.rangeStep.trim();
	const stepPart = step ? `:${step}` : '';
	return `${base}[${start}:${end}${stepPart}]`;
}

export function splitNameAndRange(raw: string): {
	name: string;
	rangeStart: string;
	rangeEnd: string;
	rangeStep: string;
} {
	const match = raw.trim().match(/^(.*)\[(-?\d*):(-?\d+)(?::(-?\d+))?\]$/);
	if (!match) {
		return { name: raw.trim(), rangeStart: '', rangeEnd: '', rangeStep: '' };
	}
	return {
		name: (match[1] ?? '').trim(),
		rangeStart: match[2] ?? '',
		rangeEnd: match[3] ?? '',
		rangeStep: match[4] ?? ''
	};
}

export function parseYamlLike(
	text: string,
	options: { skipLeafTypeValidation?: boolean } = {},
	allowedDataTypes: string[],
	nextId: () => string
): NamespaceNode[] {
	const lines = text.replace(/\r\n/g, '\n').split('\n');
	const roots: NamespaceNode[] = [];
	const stack: Array<{ indent: number; node: NamespaceNode }> = [];

	for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
		const source = lines[lineIndex];
		if (!source.trim()) continue;

		const indent = lineIndent(source);
		const line = source.trim();

		const variableMatchLegacy = line.match(/^(.+?):\s*\[([^\]]+)\]\s*$/);
		const variableMatchNewRaw = line.match(/^(.+):\s*(\S+)\s*$/);
		const variableMatchNew =
			variableMatchNewRaw &&
			(allowedDataTypes.length === 0 || allowedDataTypes.includes((variableMatchNewRaw[2] ?? '').trim()))
				? variableMatchNewRaw
				: null;
		const variableMatch = variableMatchLegacy || variableMatchNew;
		let folderMatch = line.match(/^(.+?):\s*$/);
		if (!variableMatch && !folderMatch && options.skipLeafTypeValidation && line.length > 0 && !line.includes(':')) {
			folderMatch = (line + ':').match(/^(.+?):\s*$/);
		}
		if (!variableMatch && !folderMatch) {
			throw new Error(`Invalid YAML-like line ${lineIndex + 1}: ${line}`);
		}

		if (variableMatch) {
			const typeRaw = (variableMatch[2] ?? '').trim();
			if (typeRaw && allowedDataTypes.length > 0 && !allowedDataTypes.includes(typeRaw)) {
				const rawName = (variableMatch[1] ?? '').trim();
				throw new Error(
					`Invalid YAML line ${lineIndex + 1}: "${rawName}" has unknown type ${typeRaw}. Allowed: ${allowedDataTypes.join(', ')}.`
				);
			}
			const nextIndent = nextNonEmptyIndent(lines, lineIndex);
			if (nextIndent !== null && nextIndent > indent) {
				const rawName = (variableMatch[1] ?? '').trim();
				throw new Error(
					`Invalid YAML line ${lineIndex + 1}: "${rawName}" declares a data type (${variableMatch[2].trim()}) but has indented content below. Nodes with children must be folders: use "${rawName}:" only (no type on that line).`
				);
			}
		}

		while (stack.length > 0 && stack[stack.length - 1].indent >= indent) {
			stack.pop();
		}

		const rawName = (variableMatch?.[1] ?? folderMatch?.[1] ?? '').trim();
		const parsed = splitNameAndRange(rawName);
		const node: NamespaceNode = {
			id: nextId(),
			name: parsed.name || '<New Node>',
			kind: variableMatch ? 'variable' : 'folder',
			dataType: variableMatch ? (variableMatch[2].trim() || allowedDataTypes[0] || 'Float') : null,
			rangeStart: parsed.rangeStart,
			rangeEnd: parsed.rangeEnd,
			rangeStep: parsed.rangeStep,
			children: []
		};

		const parent = stack[stack.length - 1]?.node ?? null;
		if (parent) {
			if (parent.kind === 'variable' || parent.dataType !== null) {
				throw new Error(
					`Invalid YAML line ${lineIndex + 1}: "${parsed.name || rawName}" cannot have children because it declares a data type (e.g. Float). Use a folder line (name only + colon) for nodes with children, then outdent with the tree actions if needed.`
				);
			}
			parent.children.push(node);
			setKindFromChildren(parent);
		} else {
			roots.push(node);
		}
		if (!variableMatch) {
			stack.push({ indent, node });
		}
	}

	if (!options.skipLeafTypeValidation) {
		validateNamespaceAst(roots, allowedDataTypes);
	}
	normalizeFolderState(roots);
	return roots;
}

export function normalizeFolderState(nodes: NamespaceNode[]): void {
	for (const node of nodes) {
		if (node.children.length > 0) {
			setKindFromChildren(node);
			normalizeFolderState(node.children);
		}
	}
}

export function validateNamespaceAst(nodes: NamespaceNode[], allowedDataTypes: string[], path = ''): void {
	const defaultType = allowedDataTypes[0] ?? 'Float';
	for (const node of nodes) {
		const label = path ? `${path} / ${node.name}` : node.name;
		const yamlLabel = composeNodeName(node);
		if (node.children.length > 0) {
			if (node.kind !== 'folder' || node.dataType !== null) {
				throw new Error(
					`Invalid namespace: "${label}" has children but declares a data type. Folders must use "name:" only (no type); variables must be leaves.`
				);
			}
			validateNamespaceAst(node.children, allowedDataTypes, label);
		} else {
			if (node.kind === 'folder' || node.dataType === null) {
				throw new Error(
					`Invalid namespace (leaf without type): "${label}" must declare a data type, e.g. \`${yamlLabel}: ${defaultType}\`.`
				);
			}
		}
	}
}

export function normalizeLeafFoldersToVariables(nodes: NamespaceNode[], allowedDataTypes: string[]): void {
	const defaultType = allowedDataTypes[0] ?? 'Float';
	for (const node of nodes) {
		if (node.children.length > 0) {
			normalizeLeafFoldersToVariables(node.children, allowedDataTypes);
		} else if (node.kind === 'folder' && node.dataType === null) {
			node.kind = 'variable';
			node.dataType = defaultType;
		}
	}
}

export function astToNamespaceJson(
	nodes: NamespaceNode[],
	allowedDataTypes: string[]
): Record<string, unknown> {
	const out: Record<string, unknown> = {};
	for (const node of nodes) {
		const key = composeNodeName(node);
		if (node.children.length > 0) {
			out[key] = astToNamespaceJson(node.children, allowedDataTypes);
		} else if (node.kind === 'folder') {
			out[key] = {};
		} else {
			out[key] = node.dataType ?? allowedDataTypes[0] ?? 'Float';
		}
	}
	return out;
}

export function serializeYamlLike(
	nodes: NamespaceNode[],
	indent: number,
	allowedDataTypes: string[]
): string {
	const pad = '  '.repeat(indent);
	return nodes
		.map((node) => {
			const label = composeNodeName(node);
			if (node.children.length > 0 || node.kind === 'folder') {
				const children = serializeYamlLike(node.children, indent + 1, allowedDataTypes);
				return children ? `${pad}${label}:\n${children}` : `${pad}${label}:`;
			}
			const dataType = node.dataType ?? allowedDataTypes[0] ?? 'Float';
			return `${pad}${label}: ${dataType}`;
		})
		.join('\n');
}

export function isFolderOnlyLine(source: string): boolean {
	const line = source.replace(/\r$/, '');
	const trimmed = line.trim();
	if (!trimmed) return false;
	return /^.+\s*:\s*$/.test(trimmed);
}

export function isPlainNameLine(source: string): boolean {
	const trimmed = source.replace(/\r$/, '').trim();
	return trimmed.length > 0 && !trimmed.includes(':') && !trimmed.includes('[');
}

export function mergeTrailingBlankLinesAfterAutoFill(
	beforeYaml: string,
	afterYaml: string,
	cursorLine1Based: number
): string {
	const beforeLines = beforeYaml.split('\n');
	const afterLines = afterYaml.split('\n');
	const cursorIdx = cursorLine1Based - 1;

	if (beforeLines.length > afterLines.length) {
		const merged = afterLines.slice();
		for (let i = afterLines.length; i < beforeLines.length; i += 1) {
			if (beforeLines[i].trim() !== '') break;
			merged.push('');
		}
		if (merged.length === beforeLines.length) return merged.join('\n');
	}

	if (cursorIdx < 0 || cursorIdx >= beforeLines.length) return afterYaml;
	if (beforeLines[cursorIdx].trim() !== '') return afterYaml;

	if (beforeLines.length > afterLines.length) {
		let firstDiff = -1;
		const n = Math.min(beforeLines.length, afterLines.length);
		for (let i = 0; i < n; i += 1) {
			if (beforeLines[i] !== afterLines[i]) firstDiff = i;
		}
		if (firstDiff < 0 && afterLines.length < beforeLines.length) firstDiff = afterLines.length - 1;
		if (firstDiff >= 0) {
			const merged = afterLines.slice();
			merged.splice(firstDiff + 1, 0, '');
			for (let j = cursorIdx + 1; j < beforeLines.length; j += 1) {
				if (beforeLines[j].trim() === '') merged.push('');
				else break;
			}
			while (merged.length > beforeLines.length) merged.pop();
			while (merged.length < beforeLines.length && beforeLines[merged.length]?.trim() === '') {
				merged.push('');
			}
			if (merged.length === beforeLines.length) return merged.join('\n');
		}
	}
	return afterYaml;
}

/**
 * Merge before/after YAML for auto-fill: copy line-by-line, replace only folder-only lines
 * with serialized "name: Type". Keeps other lines (e.g. plain name while typing).
 */
export function buildAutoFillMergedYaml(beforeYaml: string, afterYaml: string): string {
	const beforeNorm = beforeYaml.replace(/\r\n/g, '\n');
	const afterNorm = afterYaml.replace(/\r\n/g, '\n');
	const bl = beforeNorm.split('\n');
	const al = afterNorm.split('\n');
	const out = bl.slice();
	let j = 0;
	for (let i = 0; i < bl.length; i += 1) {
		if (j < al.length && bl[i] === al[j]) {
			out[i] = bl[i];
			j += 1;
			continue;
		}
		if (j < al.length && isFolderOnlyLine(bl[i])) {
			out[i] = al[j];
			j += 1;
			continue;
		}
		out[i] = bl[i];
		if (j < al.length && isPlainNameLine(bl[i])) j += 1;
	}
	let merged = out.join('\n');
	if (beforeNorm.endsWith('\n') && !merged.endsWith('\n')) merged += '\n';
	return merged;
}
