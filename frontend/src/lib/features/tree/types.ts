export type TreeNodeKind = "folder" | "tag";

export interface TreeNode {
  id: string;
  parentId: string | null;
  name: string;
  path: string;
  kind: TreeNodeKind;
  hasChildren: boolean;
  childIds: string[] | null;
  value?: string;
  dataType?: string;
  quality?: "good" | "uncertain" | "bad";
  timestamp?: string;
  unit?: string;
  min?: number;
  max?: number;
  options?: string[];
  maxLen?: number[];
}

export interface TreeState {
  nodes: Record<string, TreeNode>;
  rootIds: string[];
  expanded: Set<string>;
  selectedId: string | null;
  loading: Set<string>;
  errored: Set<string>;
  rootLoading: boolean;
  hasInitialized: boolean;
}

export interface VisibleTreeRow {
  id: string;
  node: TreeNode;
  depth: number;
  isExpanded: boolean;
  isLoading: boolean;
  isErrored: boolean;
}
