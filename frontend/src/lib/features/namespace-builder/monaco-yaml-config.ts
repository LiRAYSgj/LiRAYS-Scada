/**
 * Monaco editor config for the namespace YAML editor: language, theme, completion, context menu.
 */

export const UNS_YAML_LANGUAGE_ID = "uns-yaml-lite";
export const UNS_YAML_THEME_DARK_ID = "uns-yaml-dark";
export const UNS_YAML_THEME_LIGHT_ID = "uns-yaml-light";

export function getYamlLiteTokenizer(): import("monaco-editor").languages.IMonarchLanguage {
  return {
    tokenizer: {
      root: [
        [
          /^(\s*)((?:[^:\[]|\[[^\]]*\])+)(:\s*)(\S+)(\s*)$/,
          ["", "variable", "delimiter", "type", ""],
        ],
        [/^(\s*)((?:[^:\[]|\[[^\]]*\])+)(:\s*)$/, ["", "folder", "delimiter"]],
        [/\[[^\]]+\]/, "bracket"],
      ],
    },
  };
}

export function getUnsYamlDarkTheme(): import("monaco-editor").editor.IStandaloneThemeData {
  return {
    base: "vs-dark",
    inherit: true,
    rules: [
      { token: "folder", foreground: "#9CDCFE", fontStyle: "bold" },
      { token: "variable", foreground: "#DCDCAA" },
      { token: "type", foreground: "#4EC9B0" },
      { token: "delimiter", foreground: "#808080" },
      { token: "bracket", foreground: "#CE9178" },
    ],
    colors: {
      "editor.background": "#1E1E1E",
      "editor.foreground": "#D4D4D4",
    },
  };
}

export function getUnsYamlLightTheme(): import("monaco-editor").editor.IStandaloneThemeData {
  return {
    base: "vs",
    inherit: true,
    rules: [
      { token: "folder", foreground: "#0550AE", fontStyle: "bold" },
      { token: "variable", foreground: "#953800" },
      { token: "type", foreground: "#0D61BF" },
      { token: "delimiter", foreground: "#6E7681" },
      { token: "bracket", foreground: "#CF222E" },
    ],
    colors: {
      "editor.background": "#FFFFFF",
      "editor.foreground": "#24292F",
    },
  };
}

/** Theme id for the given app color mode. */
export function getMonacoThemeId(colorMode: "light" | "dark"): string {
  return colorMode === "dark"
    ? UNS_YAML_THEME_DARK_ID
    : UNS_YAML_THEME_LIGHT_ID;
}

export function getYamlCompletionProvider(
  monaco: typeof import("monaco-editor"),
  allowedDataTypes: string[],
) {
  return {
    triggerCharacters: [":", " "],
    provideCompletionItems(
      model: import("monaco-editor").editor.ITextModel,
      position: import("monaco-editor").Position,
    ) {
      const lineContent = model.getLineContent(position.lineNumber);
      const lineUntilCursor = lineContent.substring(0, position.column - 1);
      const match = lineUntilCursor.match(/:\s*(\S*)$/);
      if (!match) return { suggestions: [] };
      const lineIndentStr = lineContent.match(/^[ \t]*/)?.[0] ?? "";
      const currentIndent = Math.floor(
        lineIndentStr.replace(/\t/g, "  ").length / 2,
      );
      for (
        let ln = position.lineNumber + 1;
        ln <= model.getLineCount();
        ln += 1
      ) {
        const nextLine = model.getLineContent(ln);
        if (!nextLine.trim()) continue;
        const nextIndentStr = nextLine.match(/^[ \t]*/)?.[0] ?? "";
        const nextIndent = Math.floor(
          nextIndentStr.replace(/\t/g, "  ").length / 2,
        );
        if (nextIndent > currentIndent) return { suggestions: [] };
        break;
      }
      const partial = (match[1] ?? "").trim();
      const afterColon = match[0];
      const needsSpaceBeforeType = afterColon === ":";
      const typeStartColumn = position.column - partial.length;
      const range = {
        startLineNumber: position.lineNumber,
        startColumn: typeStartColumn,
        endLineNumber: position.lineNumber,
        endColumn: position.column,
      };
      const types = allowedDataTypes.filter((t) =>
        partial ? t.toLowerCase().startsWith(partial.toLowerCase()) : true,
      );
      return {
        suggestions: types.map((type: string) => ({
          label: type,
          kind: monaco.languages.CompletionItemKind.EnumMember,
          insertText: (needsSpaceBeforeType ? " " : "") + type,
          range,
        })),
      };
    },
  };
}

/** Remove Command Palette and default Cut/Copy/Paste from editor context menu via Monaco internal API. */
export async function removeEditorContextMenuItems(
  monaco: typeof import("monaco-editor"),
): Promise<void> {
  try {
    const actions =
      await import("monaco-editor/esm/vs/platform/actions/common/actions.js");
    const { MenuRegistry, MenuId } = actions as {
      MenuRegistry?: { _menuItems?: Map<unknown, unknown> };
      MenuId?: { EditorContext?: unknown };
    };
    const list = MenuRegistry?._menuItems?.get(MenuId?.EditorContext) as
      | {
          size: number;
          _first: { next: unknown; element?: { command?: { id?: string } } };
          _last: { next: unknown };
          _remove: (n: unknown) => void;
        }
      | undefined;
    if (!list || list.size === 0) return;
    const removableIds = [
      "editor.action.quickCommand",
      "editor.action.commandPalette",
      "editor.action.clipboardCutAction",
      "editor.action.clipboardCopyAction",
      "editor.action.clipboardPasteAction",
    ];
    let node: unknown = list._first;
    const end = list._last?.next;
    while (node && node !== end) {
      const current = node as {
        next: unknown;
        element?: { command?: { id?: string } };
      };
      const nextNode = current.next;
      if (
        current.element?.command?.id &&
        removableIds.includes(current.element.command.id)
      ) {
        list._remove(node);
      }
      node = nextNode;
    }
  } catch {
    /* MenuRegistry API may change; CSS/JS prune fallback still applied */
  }
}
