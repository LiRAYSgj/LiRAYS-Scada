/** Drag payload carrying a namespace path from the variable tree into form fields. */
export const TREE_PATH_DRAG_MIME = "application/x-lirays-path";
/** Companion payload: `"folder"` when the dragged tree node is a folder. */
export const TREE_PATH_KIND_DRAG_MIME = "application/x-lirays-path-kind";

export interface TreePathDrop {
  path: string;
  isFolder: boolean;
}

export function hasTreePath(event: DragEvent): boolean {
  return event.dataTransfer?.types.includes(TREE_PATH_DRAG_MIME) ?? false;
}

export function readTreePath(event: DragEvent): TreePathDrop | null {
  const path = event.dataTransfer?.getData(TREE_PATH_DRAG_MIME)?.trim();
  if (!path) {
    return null;
  }
  return {
    path,
    isFolder:
      event.dataTransfer?.getData(TREE_PATH_KIND_DRAG_MIME) === "folder",
  };
}

/** Pattern matching every variable below `folder` at any depth, including future ones. */
export function folderPattern(folder: string): string {
  const trimmed = folder.replace(/\/+$/, "");
  return `${trimmed}/**`;
}
