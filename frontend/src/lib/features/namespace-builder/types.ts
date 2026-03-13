/**
 * Types for the unified namespace builder (visual tree + YAML code editor).
 */

export type EditorMode = 'visual-tree' | 'code-yaml';

export type NodeKind = 'folder' | 'variable';

export type NamespaceNode = {
	id: string;
	name: string;
	kind: NodeKind;
	dataType: string | null;
	rangeStart: string;
	rangeEnd: string;
	rangeStep: string;
	children: NamespaceNode[];
};

export type FlatRow = {
	id: string;
	depth: number;
	node: NamespaceNode;
	parentId: string | null;
	parentChildren: NamespaceNode[];
	index: number;
};
