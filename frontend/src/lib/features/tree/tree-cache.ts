import type { TreeState } from "./types";

let cachedTreeState: TreeState | null = null;

export function readTreeStateCache(): TreeState | null {
  if (!cachedTreeState) {
    return null;
  }
  return structuredClone(cachedTreeState);
}

export function writeTreeStateCache(state: TreeState): void {
  cachedTreeState = structuredClone(state);
}

export function clearTreeStateCache(): void {
  cachedTreeState = null;
}
