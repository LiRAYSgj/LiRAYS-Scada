/**
 * YAML-like parse/serialize and namespace AST helpers for the namespace builder.
 * Format: "name: Type" (variable) or "name:" (folder).
 * Names support one series expression: numeric [start?:end:step?] or enum [A, B, C],
 * with optional prefix/suffix around the bracket expression.
 */

import type { NamespaceNode } from "./types.js";

const VARIABLE_BLOCK_KEYS = new Set([
  "type",
  "unit",
  "min",
  "max",
  "maxlength",
  "options",
  "default",
  "writable",
  "description",
]);

export function lineIndent(source: string): number {
  const indentStr = source.match(/^[ \t]*/)?.[0] ?? "";
  return Math.floor(indentStr.replace(/\t/g, "  ").length / 2);
}

export function nextNonEmptyIndent(
  lines: string[],
  afterIndex: number,
): number | null {
  for (let i = afterIndex + 1; i < lines.length; i += 1) {
    if (lines[i].trim()) return lineIndent(lines[i]);
  }
  return null;
}

export function setKindFromChildren(node: NamespaceNode): void {
  if (node.children.length > 0) {
    node.kind = "folder";
    node.dataType = null;
    node.unit = "";
    node.min = "";
    node.max = "";
    node.maxLength = "";
    node.options = [];
  }
}

export function composeNodeName(node: NamespaceNode): string {
  const base = node.name.trim();
  const suffix = (node.nameSuffix ?? "").trim();
  if (node.seriesMode === "enum") {
    const values = (node.seriesValues ?? "").trim();
    return values ? `${base}[${values}]${suffix}` : `${base}${suffix}`;
  }
  if (node.seriesMode === "numeric") {
    const end = node.rangeEnd.trim();
    if (!end) return `${base}${suffix}`;
    const start = node.rangeStart.trim();
    const step = node.rangeStep.trim();
    const stepPart = step ? `:${step}` : "";
    return `${base}[${start}:${end}${stepPart}]${suffix}`;
  }
  return `${base}${suffix}`;
}

export function splitNameAndRange(raw: string): {
  name: string;
  nameSuffix: string;
  seriesMode: "none" | "numeric" | "enum";
  seriesValues: string;
  rangeStart: string;
  rangeEnd: string;
  rangeStep: string;
} {
  const value = raw.trim();
  const firstOpen = value.indexOf("[");
  const firstClose = value.indexOf("]");
  const secondOpen = firstOpen >= 0 ? value.indexOf("[", firstOpen + 1) : -1;
  const secondClose = firstClose >= 0 ? value.indexOf("]", firstClose + 1) : -1;

  if (
    firstOpen < 0 ||
    firstClose < firstOpen ||
    secondOpen >= 0 ||
    secondClose >= 0
  ) {
    return {
      name: value,
      nameSuffix: "",
      seriesMode: "none",
      seriesValues: "",
      rangeStart: "",
      rangeEnd: "",
      rangeStep: "",
    };
  }

  const prefix = value.slice(0, firstOpen).trim();
  const body = value.slice(firstOpen + 1, firstClose).trim();
  const suffix = value.slice(firstClose + 1).trim();
  const numeric = body.match(/^(-?\d*):(-?\d+)(?::(-?\d+))?$/);
  if (numeric) {
    return {
      name: prefix,
      nameSuffix: suffix,
      seriesMode: "numeric",
      seriesValues: "",
      rangeStart: numeric[1] ?? "",
      rangeEnd: numeric[2] ?? "",
      rangeStep: numeric[3] ?? "",
    };
  }

  return {
    name: prefix,
    nameSuffix: suffix,
    seriesMode: "enum",
    seriesValues: body,
    rangeStart: "",
    rangeEnd: "",
    rangeStep: "",
  };
}

function splitKeyAndValue(line: string): { key: string; value: string | null } | null {
  let depth = 0;
  for (let i = 0; i < line.length; i += 1) {
    const ch = line[i];
    if (ch === "[") depth += 1;
    else if (ch === "]") depth = Math.max(0, depth - 1);
    else if (ch === ":" && depth === 0) {
      const key = line.slice(0, i).trim();
      const value = line.slice(i + 1).trim();
      if (!key) return null;
      return { key, value: value.length > 0 ? value : null };
    }
  }
  return null;
}

export function parseYamlLike(
  text: string,
  options: { skipLeafTypeValidation?: boolean } = {},
  allowedDataTypes: string[],
  nextId: () => string,
): NamespaceNode[] {
  const lines = text.replace(/\r\n/g, "\n").split("\n");
  const roots: NamespaceNode[] = [];
  const stack: Array<{ indent: number; node: NamespaceNode }> = [];

  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const source = lines[lineIndex];
    if (!source.trim()) continue;

    const indent = lineIndent(source);
    const line = source.trim();

    while (stack.length > 0 && stack[stack.length - 1].indent >= indent) {
      stack.pop();
    }
    const parent = stack[stack.length - 1]?.node ?? null;

    const listItem = line.match(/^-+\s+(.+)$/);
    if (listItem) {
      if (!parent || parent.name.trim().toLowerCase() !== "options") {
        throw new Error(
          `Invalid YAML-like line ${lineIndex + 1}: list entries are only allowed under "options:"`,
        );
      }
      const optionValue = (listItem[1] ?? "").trim();
      if (!optionValue) {
        throw new Error(
          `Invalid YAML-like line ${lineIndex + 1}: option entry cannot be empty`,
        );
      }
      parent.children.push({
        id: nextId(),
        name: optionValue,
        nameSuffix: "",
        seriesMode: "none",
        seriesValues: "",
        kind: "variable",
        dataType: "Text",
        unit: "",
        min: "",
        max: "",
        maxLength: "",
        options: [],
        rangeStart: "",
        rangeEnd: "",
        rangeStep: "",
        children: [],
      });
      continue;
    }

    let parsedLine = splitKeyAndValue(line);
    if (!parsedLine && options.skipLeafTypeValidation && !line.includes(":")) {
      parsedLine = { key: line.trim(), value: null };
    }
    if (!parsedLine) {
      throw new Error(`Invalid YAML-like line ${lineIndex + 1}: ${line}`);
    }
    const key = parsedLine.key;
    const value = parsedLine.value;
    const isVariableDeclaration = value !== null;
    const isVariableBlockKey = VARIABLE_BLOCK_KEYS.has(key.trim().toLowerCase());

    if (isVariableDeclaration) {
      const typeRaw = value?.trim() ?? "";
      if (
        !isVariableBlockKey &&
        typeRaw &&
        allowedDataTypes.length > 0 &&
        !allowedDataTypes.includes(typeRaw)
      ) {
        const rawName = key;
        throw new Error(
          `Invalid YAML line ${lineIndex + 1}: "${rawName}" has unknown type ${typeRaw}. Allowed: ${allowedDataTypes.join(", ")}.`,
        );
      }
      const nextIndent = nextNonEmptyIndent(lines, lineIndex);
      if (!isVariableBlockKey && nextIndent !== null && nextIndent > indent) {
        const rawName = key;
        throw new Error(
          `Invalid YAML line ${lineIndex + 1}: "${rawName}" declares a data type (${typeRaw}) but has indented content below. Nodes with children must be folders: use "${rawName}:" only (no type on that line).`,
        );
      }
    }

    const rawName = key;
    const parsed = splitNameAndRange(rawName);
    const parsedName =
      parsed.name.trim() === "" && parsed.seriesMode === "none"
        ? "<New Node>"
        : parsed.name;
    const node: NamespaceNode = {
      id: nextId(),
      name: parsedName,
      nameSuffix: parsed.nameSuffix,
      seriesMode: parsed.seriesMode,
      seriesValues: parsed.seriesValues,
      kind: isVariableDeclaration ? "variable" : "folder",
      dataType: isVariableDeclaration
        ? value?.trim() || allowedDataTypes[0] || "Float"
        : null,
      unit: "",
      min: "",
      max: "",
      maxLength: "",
      options: [],
      rangeStart: parsed.rangeStart,
      rangeEnd: parsed.rangeEnd,
      rangeStep: parsed.rangeStep,
      children: [],
    };

    if (parent) {
      if (parent.kind === "variable" || parent.dataType !== null) {
        throw new Error(
          `Invalid YAML line ${lineIndex + 1}: "${parsed.name || rawName}" cannot have children because it declares a data type (e.g. Float). Use a folder line (name only + colon) for nodes with children, then outdent with the tree actions if needed.`,
        );
      }
      parent.children.push(node);
      setKindFromChildren(parent);
    } else {
      roots.push(node);
    }
    if (!isVariableDeclaration) {
      stack.push({ indent, node });
    }
  }

  normalizeVariableBlockNodes(roots, allowedDataTypes);
  if (!options.skipLeafTypeValidation) {
    validateNamespaceAst(roots, allowedDataTypes);
  }
  normalizeFolderState(roots);
  return roots;
}

export function normalizeVariableBlockNodes(
  nodes: NamespaceNode[],
  allowedDataTypes: string[],
): void {
  function parseOptionValues(optionNode: NamespaceNode): string[] {
    if (optionNode.children.length > 0) {
      return optionNode.children
        .map((child) => child.name.trim())
        .filter((value) => value.length > 0);
    }
    return (optionNode.dataType ?? "")
      .split(",")
      .map((value) => value.trim())
      .filter((value) => value.length > 0);
  }

  for (const node of nodes) {
    if (node.children.length === 0) continue;

    const childKeySet = new Set(
      node.children.map((child) => child.name.trim().toLowerCase()),
    );
    const isVariableBlock =
      childKeySet.size > 0 &&
      Array.from(childKeySet).every((key) => VARIABLE_BLOCK_KEYS.has(key));

    if (!isVariableBlock) {
      normalizeVariableBlockNodes(node.children, allowedDataTypes);
      continue;
    }

    const typeNode = node.children.find(
      (child) => child.name.trim().toLowerCase() === "type",
    );
    const rawType = (typeNode?.dataType ?? "").trim();
    if (!rawType) {
      throw new Error(
        `Invalid variable block "${node.name}": missing required "type" property.`,
      );
    }
    if (allowedDataTypes.length > 0 && !allowedDataTypes.includes(rawType)) {
      throw new Error(
        `Invalid variable block "${node.name}": unknown type ${rawType}. Allowed: ${allowedDataTypes.join(", ")}.`,
      );
    }

    node.kind = "variable";
    node.dataType = rawType;
    const unitNode = node.children.find(
      (child) => child.name.trim().toLowerCase() === "unit",
    );
    const minNode = node.children.find(
      (child) => child.name.trim().toLowerCase() === "min",
    );
    const maxNode = node.children.find(
      (child) => child.name.trim().toLowerCase() === "max",
    );
    const maxLengthNode = node.children.find(
      (child) => child.name.trim().toLowerCase() === "maxlength",
    );
    const optionsNode = node.children.find(
      (child) => child.name.trim().toLowerCase() === "options",
    );

    node.unit = (unitNode?.dataType ?? "").trim();
    node.min = (minNode?.dataType ?? "").trim();
    node.max = (maxNode?.dataType ?? "").trim();
    node.maxLength = (maxLengthNode?.dataType ?? "").trim();
    node.options = optionsNode ? parseOptionValues(optionsNode) : [];
    node.children = [];
  }
}

export function normalizeFolderState(nodes: NamespaceNode[]): void {
  for (const node of nodes) {
    if (node.children.length > 0) {
      setKindFromChildren(node);
      normalizeFolderState(node.children);
    }
  }
}

export function validateNamespaceAst(
  nodes: NamespaceNode[],
  allowedDataTypes: string[],
  path = "",
): void {
  const isNumericLiteral = (raw: string): boolean =>
    /^[-+]?(?:\d+\.?\d*|\.\d+)$/.test(raw.trim());
  const isIntegerLiteral = (raw: string): boolean =>
    /^[-+]?\d+$/.test(raw.trim());
  const defaultType = allowedDataTypes[0] ?? "Float";
  for (const node of nodes) {
    const label = path ? `${path} / ${node.name}` : node.name;
    const yamlLabel = composeNodeName(node);
    if (node.children.length > 0) {
      if (node.kind !== "folder" || node.dataType !== null) {
        throw new Error(
          `Invalid namespace: "${label}" has children but declares a data type. Folders must use "name:" only (no type); variables must be leaves.`,
        );
      }
      validateNamespaceAst(node.children, allowedDataTypes, label);
    } else {
      if (node.kind === "folder" || node.dataType === null) {
        throw new Error(
          `Invalid namespace (leaf without type): "${label}" must declare a data type, e.g. \`${yamlLabel}: ${defaultType}\`.`,
        );
      }
      const dataType = node.dataType.toLowerCase();
      const min = node.min.trim();
      const max = node.max.trim();
      const maxLength = node.maxLength.trim();
      const hasOptions = node.options.length > 0;
      const hasUnit = node.unit.trim() !== "";
      const isText = dataType === "text" || dataType === "string";
      const isBoolean = dataType === "boolean" || dataType === "bool";
      const isInteger = dataType === "integer" || dataType === "int";
      const isFloat = dataType === "float";
      const isNumericType = isInteger || isFloat;

      if (isText) {
        if (min) {
          throw new Error(
            `Invalid namespace: "${label}" is Text and cannot define Min.`,
          );
        }
        if (max) {
          throw new Error(
            `Invalid namespace: "${label}" is Text and cannot define Max.`,
          );
        }
        if (maxLength && !isIntegerLiteral(maxLength)) {
          throw new Error(
            `Invalid namespace: "${label}" Max Length must be an integer.`,
          );
        }
        if (hasUnit) {
          throw new Error(
            `Invalid namespace: "${label}" is Text and cannot define Unit.`,
          );
        }
      } else if (isNumericType) {
        if (min && !isNumericLiteral(min)) {
          throw new Error(
            `Invalid namespace: "${label}" Min must be numeric.`,
          );
        }
        if (max && !isNumericLiteral(max)) {
          throw new Error(
            `Invalid namespace: "${label}" Max must be numeric.`,
          );
        }
        if (maxLength) {
          throw new Error(
            `Invalid namespace: "${label}" ${node.dataType} cannot define Max Length.`,
          );
        }
        if (hasOptions) {
          throw new Error(
            `Invalid namespace: "${label}" ${node.dataType} cannot define Options.`,
          );
        }
        if (isInteger) {
          if (min && !isIntegerLiteral(min)) {
            throw new Error(
              `Invalid namespace: "${label}" Integer Min must be an integer.`,
            );
          }
          if (max && !isIntegerLiteral(max)) {
            throw new Error(
              `Invalid namespace: "${label}" Integer Max must be an integer.`,
            );
          }
        }
      } else if (isBoolean) {
        if (min || max || maxLength || hasOptions || hasUnit) {
          throw new Error(
            `Invalid namespace: "${label}" Boolean cannot define Unit/Min/Max/Max Length/Options.`,
          );
        }
      } else {
        if (min && !isNumericLiteral(min)) {
          throw new Error(
            `Invalid namespace: "${label}" Min must be numeric.`,
          );
        }
        if (max && !isNumericLiteral(max)) {
          throw new Error(
            `Invalid namespace: "${label}" Max must be numeric.`,
          );
        }
        if (maxLength && !isIntegerLiteral(maxLength)) {
          throw new Error(
            `Invalid namespace: "${label}" Max Length must be an integer.`,
          );
        }
      }
    }
  }
}

export function normalizeLeafFoldersToVariables(
  nodes: NamespaceNode[],
  allowedDataTypes: string[],
): void {
  const defaultType = allowedDataTypes[0] ?? "Float";
  for (const node of nodes) {
    if (node.children.length > 0) {
      normalizeLeafFoldersToVariables(node.children, allowedDataTypes);
    } else if (node.kind === "folder" && node.dataType === null) {
      node.kind = "variable";
      node.dataType = defaultType;
    }
  }
}

export function astToNamespaceJson(
  nodes: NamespaceNode[],
  allowedDataTypes: string[],
): Record<string, unknown> {
  const parseFiniteNumber = (raw: string): number | string => {
    const parsed = Number(raw);
    return Number.isFinite(parsed) ? parsed : raw;
  };

  const out: Record<string, unknown> = {};
  for (const node of nodes) {
    const key = composeNodeName(node);
    if (node.children.length > 0) {
      out[key] = astToNamespaceJson(node.children, allowedDataTypes);
    } else if (node.kind === "folder") {
      out[key] = {};
    } else {
      const dataType = node.dataType ?? allowedDataTypes[0] ?? "Float";
      const hasMetadata =
        node.unit.trim() !== "" ||
        node.min.trim() !== "" ||
        node.max.trim() !== "" ||
        node.maxLength.trim() !== "" ||
        node.options.length > 0;
      if (!hasMetadata) {
        out[key] = dataType;
        continue;
      }
      const variable: Record<string, unknown> = { type: dataType };
      if (node.unit.trim() !== "") variable.unit = node.unit.trim();
      if (node.min.trim() !== "") variable.min = parseFiniteNumber(node.min);
      if (node.max.trim() !== "") variable.max = parseFiniteNumber(node.max);
      if (node.maxLength.trim() !== "") {
        variable.maxLength = parseFiniteNumber(node.maxLength);
      }
      if (node.options.length > 0) variable.options = [...node.options];
      out[key] = variable;
    }
  }
  return out;
}

export function serializeYamlLike(
  nodes: NamespaceNode[],
  indent: number,
  allowedDataTypes: string[],
): string {
  const pad = "  ".repeat(indent);
  return nodes
    .map((node) => {
      const label = composeNodeName(node);
      if (node.children.length > 0 || node.kind === "folder") {
        const children = serializeYamlLike(
          node.children,
          indent + 1,
          allowedDataTypes,
        );
        return children ? `${pad}${label}:\n${children}` : `${pad}${label}:`;
      }
      const dataType = node.dataType ?? allowedDataTypes[0] ?? "Float";
      const hasMetadata =
        node.unit.trim() !== "" ||
        node.min.trim() !== "" ||
        node.max.trim() !== "" ||
        node.maxLength.trim() !== "" ||
        node.options.length > 0;
      if (!hasMetadata) {
        return `${pad}${label}: ${dataType}`;
      }
      const lines: string[] = [`${pad}${label}:`, `${pad}  type: ${dataType}`];
      if (node.unit.trim() !== "") lines.push(`${pad}  unit: ${node.unit.trim()}`);
      if (node.min.trim() !== "") lines.push(`${pad}  min: ${node.min.trim()}`);
      if (node.max.trim() !== "") lines.push(`${pad}  max: ${node.max.trim()}`);
      if (node.maxLength.trim() !== "") {
        lines.push(`${pad}  maxLength: ${node.maxLength.trim()}`);
      }
      if (node.options.length > 0) {
        lines.push(`${pad}  options:`);
        for (const option of node.options) {
          lines.push(`${pad}    - ${option}`);
        }
      }
      return lines.join("\n");
    })
    .join("\n");
}

export function isFolderOnlyLine(source: string): boolean {
  const line = source.replace(/\r$/, "");
  const trimmed = line.trim();
  if (!trimmed) return false;
  return /^.+\s*:\s*$/.test(trimmed);
}

export function isPlainNameLine(source: string): boolean {
  const trimmed = source.replace(/\r$/, "").trim();
  return trimmed.length > 0 && !trimmed.includes(":") && !trimmed.includes("[");
}

export function mergeTrailingBlankLinesAfterAutoFill(
  beforeYaml: string,
  afterYaml: string,
  cursorLine1Based: number,
): string {
  const beforeLines = beforeYaml.split("\n");
  const afterLines = afterYaml.split("\n");
  const cursorIdx = cursorLine1Based - 1;

  if (beforeLines.length > afterLines.length) {
    const merged = afterLines.slice();
    for (let i = afterLines.length; i < beforeLines.length; i += 1) {
      if (beforeLines[i].trim() !== "") break;
      merged.push("");
    }
    if (merged.length === beforeLines.length) return merged.join("\n");
  }

  if (cursorIdx < 0 || cursorIdx >= beforeLines.length) return afterYaml;
  if (beforeLines[cursorIdx].trim() !== "") return afterYaml;

  if (beforeLines.length > afterLines.length) {
    let firstDiff = -1;
    const n = Math.min(beforeLines.length, afterLines.length);
    for (let i = 0; i < n; i += 1) {
      if (beforeLines[i] !== afterLines[i]) firstDiff = i;
    }
    if (firstDiff < 0 && afterLines.length < beforeLines.length)
      firstDiff = afterLines.length - 1;
    if (firstDiff >= 0) {
      const merged = afterLines.slice();
      merged.splice(firstDiff + 1, 0, "");
      for (let j = cursorIdx + 1; j < beforeLines.length; j += 1) {
        if (beforeLines[j].trim() === "") merged.push("");
        else break;
      }
      while (merged.length > beforeLines.length) merged.pop();
      while (
        merged.length < beforeLines.length &&
        beforeLines[merged.length]?.trim() === ""
      ) {
        merged.push("");
      }
      if (merged.length === beforeLines.length) return merged.join("\n");
    }
  }
  return afterYaml;
}

/**
 * Merge before/after YAML for auto-fill: copy line-by-line, replace only folder-only lines
 * with serialized "name: Type". Keeps other lines (e.g. plain name while typing).
 */
export function buildAutoFillMergedYaml(
  beforeYaml: string,
  afterYaml: string,
): string {
  const beforeNorm = beforeYaml.replace(/\r\n/g, "\n");
  const afterNorm = afterYaml.replace(/\r\n/g, "\n");
  const bl = beforeNorm.split("\n");
  const al = afterNorm.split("\n");
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
  let merged = out.join("\n");
  if (beforeNorm.endsWith("\n") && !merged.endsWith("\n")) merged += "\n";
  return merged;
}
