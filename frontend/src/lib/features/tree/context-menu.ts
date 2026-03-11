import type { Readable } from "svelte/store";
import type { TreeNode, TreeNodeKind } from "./types";

export interface MenuContext {
  node: TreeNode;
  event: MouseEvent;
  kind: "node" | "drop";
}

export interface MenuOption {
  id: string;
  label: string;
  disabled?: boolean;
  onSelect?: (context: MenuContext) => void | Promise<void>;
  children?: MenuOption[];
  getChildren?: MenuOptionsResolver;
}

export type MenuOptionsInput =
  | MenuOption[]
  | Promise<MenuOption[]>
  | Readable<MenuOption[]>;
export type MenuOptionsResolver = (context: MenuContext) => MenuOptionsInput;
export type MenuResolverByKind = Record<TreeNodeKind, MenuOptionsResolver>;

function isReadableStore(value: unknown): value is Readable<MenuOption[]> {
  return typeof value === "object" && value !== null && "subscribe" in value;
}

function firstStoreValue(store: Readable<MenuOption[]>): Promise<MenuOption[]> {
  return new Promise((resolve) => {
    let unsubscribe = () => {};
    unsubscribe = store.subscribe((value) => {
      resolve(value);
      unsubscribe();
    });
  });
}

export async function resolveMenuOptions(
  input: MenuOptionsInput,
): Promise<MenuOption[]> {
  if (Array.isArray(input)) {
    return input;
  }

  if (isReadableStore(input)) {
    return firstStoreValue(input);
  }

  return input;
}
