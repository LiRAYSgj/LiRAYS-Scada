import { beforeEach, describe, expect, it } from "vitest";
import type { NamespaceNode } from "./types.js";
import {
  lineIndent,
  nextNonEmptyIndent,
  splitNameAndRange,
  composeNodeName,
  setKindFromChildren,
  isFolderOnlyLine,
  isPlainNameLine,
  parseYamlLike,
  serializeYamlLike,
  validateNamespaceAst,
  normalizeLeafFoldersToVariables,
  normalizeFolderState,
  astToNamespaceJson,
  mergeTrailingBlankLinesAfterAutoFill,
  buildAutoFillMergedYaml,
} from "./namespace-yaml.js";

const ALLOWED_TYPES = ["Float", "Integer", "Text", "Boolean"];
let idCounter = 0;
const nextId = () => `id-${idCounter++}`;

function resetId(): void {
  idCounter = 0;
}

describe("namespace-yaml", () => {
  describe("lineIndent", () => {
    it("returns 0 for no indent", () => {
      expect(lineIndent("foo:")).toBe(0);
      expect(lineIndent("x: Float")).toBe(0);
    });
    it("counts 2 spaces as 1 indent level", () => {
      expect(lineIndent("  bar:")).toBe(1);
      expect(lineIndent("    baz: Float")).toBe(2);
    });
    it("treats tab as 2 spaces", () => {
      expect(lineIndent("\tfoo:")).toBe(1);
    });
  });

  describe("nextNonEmptyIndent", () => {
    it("returns next non-empty line indent", () => {
      const lines = ["  a:", "", "    b: Float"];
      expect(nextNonEmptyIndent(lines, 0)).toBe(2);
    });
    it("returns null when no more non-empty lines", () => {
      const lines = ["  a:", "", ""];
      expect(nextNonEmptyIndent(lines, 0)).toBe(null);
    });
  });

  describe("splitNameAndRange", () => {
    it("returns plain name when no brackets", () => {
      expect(splitNameAndRange("Motor_")).toEqual({
        name: "Motor_",
        rangeStart: "",
        rangeEnd: "",
        rangeStep: "",
      });
    });
    it("parses range [0:20]", () => {
      expect(splitNameAndRange("Area_[0:20]")).toEqual({
        name: "Area_",
        rangeStart: "0",
        rangeEnd: "20",
        rangeStep: "",
      });
    });
    it("parses range [:4] (empty start)", () => {
      expect(splitNameAndRange("Line_[:4]")).toEqual({
        name: "Line_",
        rangeStart: "",
        rangeEnd: "4",
        rangeStep: "",
      });
    });
    it("parses range with step [0:100:2]", () => {
      expect(splitNameAndRange("Var_[0:100:2]")).toEqual({
        name: "Var_",
        rangeStart: "0",
        rangeEnd: "100",
        rangeStep: "2",
      });
    });
  });

  describe("composeNodeName", () => {
    it("returns name when no range", () => {
      expect(
        composeNodeName({
          name: "Power",
          rangeEnd: "",
          rangeStart: "",
          rangeStep: "",
        } as NamespaceNode),
      ).toBe("Power");
    });
    it("returns name[start:end] when range", () => {
      expect(
        composeNodeName({
          name: "Area_",
          rangeStart: "0",
          rangeEnd: "20",
          rangeStep: "",
        } as NamespaceNode),
      ).toBe("Area_[0:20]");
    });
    it("includes step when present", () => {
      expect(
        composeNodeName({
          name: "Var_",
          rangeStart: "0",
          rangeEnd: "100",
          rangeStep: "2",
        } as NamespaceNode),
      ).toBe("Var_[0:100:2]");
    });
  });

  describe("setKindFromChildren", () => {
    it("sets kind to folder and dataType to null when node has children", () => {
      const node: NamespaceNode = {
        id: "1",
        name: "F",
        kind: "variable",
        dataType: "Float",
        rangeStart: "",
        rangeEnd: "",
        rangeStep: "",
        children: [
          {
            id: "2",
            name: "C",
            kind: "variable",
            dataType: "Float",
            rangeStart: "",
            rangeEnd: "",
            rangeStep: "",
            children: [],
          },
        ],
      };
      setKindFromChildren(node);
      expect(node.kind).toBe("folder");
      expect(node.dataType).toBeNull();
    });
  });

  describe("isFolderOnlyLine", () => {
    it('returns true for "name:"', () => {
      expect(isFolderOnlyLine("Area_:")).toBe(true);
      expect(isFolderOnlyLine("  Motor_:")).toBe(true);
    });
    it('returns false for "name: Type"', () => {
      expect(isFolderOnlyLine("Power: Float")).toBe(false);
    });
    it("returns false for blank or plain name", () => {
      expect(isFolderOnlyLine("")).toBe(false);
      expect(isFolderOnlyLine("  ")).toBe(false);
      expect(isFolderOnlyLine("abc")).toBe(false);
    });
  });

  describe("isPlainNameLine", () => {
    it("returns true for line without colon or bracket", () => {
      expect(isPlainNameLine("abc")).toBe(true);
      expect(isPlainNameLine("  typing")).toBe(true);
    });
    it('returns false for "name:" or "name: Type"', () => {
      expect(isPlainNameLine("x:")).toBe(false);
      expect(isPlainNameLine("x: Float")).toBe(false);
    });
    it("returns false for name with range", () => {
      expect(isPlainNameLine("Var_[0:10]")).toBe(false);
    });
  });

  describe("parseYamlLike", () => {
    beforeEach(resetId);

    it("parses single variable line", () => {
      const roots = parseYamlLike("Power: Float", {}, ALLOWED_TYPES, nextId);
      expect(roots).toHaveLength(1);
      expect(roots[0].name).toBe("Power");
      expect(roots[0].kind).toBe("variable");
      expect(roots[0].dataType).toBe("Float");
      expect(roots[0].children).toHaveLength(0);
    });

    it("parses folder with children", () => {
      const yaml = [
        "Area_:",
        "  Line_:",
        "    Motor_[:2]:",
        "      Power: Float",
      ].join("\n");
      const roots = parseYamlLike(yaml, {}, ALLOWED_TYPES, nextId);
      expect(roots).toHaveLength(1);
      expect(roots[0].name).toBe("Area_");
      expect(roots[0].kind).toBe("folder");
      expect(roots[0].children).toHaveLength(1);
      expect(roots[0].children[0].name).toBe("Line_");
      expect(roots[0].children[0].children[0].name).toBe("Motor_");
      expect(roots[0].children[0].children[0].rangeEnd).toBe("2");
      const power = roots[0].children[0].children[0].children[0];
      expect(power.name).toBe("Power");
      expect(power.dataType).toBe("Float");
    });

    it("parses range notation in variable name", () => {
      const roots = parseYamlLike(
        "Var_[0:100]: Float",
        {},
        ALLOWED_TYPES,
        nextId,
      );
      expect(roots[0].name).toBe("Var_");
      expect(roots[0].rangeStart).toBe("0");
      expect(roots[0].rangeEnd).toBe("100");
    });

    it("accepts plain name as folder when skipLeafTypeValidation", () => {
      const roots = parseYamlLike(
        "typing",
        { skipLeafTypeValidation: true },
        ALLOWED_TYPES,
        nextId,
      );
      expect(roots).toHaveLength(1);
      expect(roots[0].name).toBe("typing");
      expect(roots[0].kind).toBe("folder");
    });

    it("throws on invalid type", () => {
      expect(() =>
        parseYamlLike("X: UnknownType", {}, ALLOWED_TYPES, nextId),
      ).toThrow(/Invalid YAML-like line|unknown type/);
    });

    it("throws on variable with indented children", () => {
      const yaml = "Motor_: Float\n  Power: Float";
      expect(() => parseYamlLike(yaml, {}, ALLOWED_TYPES, nextId)).toThrow(
        /declares a data type.*indented content/,
      );
    });

    it("throws on leaf without type when strict", () => {
      const yaml = "Folder_:";
      expect(() => parseYamlLike(yaml, {}, ALLOWED_TYPES, nextId)).toThrow(
        /leaf without type|must declare a data type/,
      );
    });

    it("returns empty array for empty or whitespace-only input", () => {
      resetId();
      const roots1 = parseYamlLike("", {}, ALLOWED_TYPES, nextId);
      expect(roots1).toHaveLength(0);
      resetId();
      const roots2 = parseYamlLike("\n  \n", {}, ALLOWED_TYPES, nextId);
      expect(roots2).toHaveLength(0);
    });
  });

  describe("serializeYamlLike", () => {
    it("serializes variable", () => {
      const nodes: NamespaceNode[] = [
        {
          id: "1",
          name: "Power",
          kind: "variable",
          dataType: "Float",
          rangeStart: "",
          rangeEnd: "",
          rangeStep: "",
          children: [],
        },
      ];
      expect(serializeYamlLike(nodes, 0, ALLOWED_TYPES)).toBe("Power: Float");
    });

    it("serializes folder with children", () => {
      const nodes: NamespaceNode[] = [
        {
          id: "1",
          name: "Area_",
          kind: "folder",
          dataType: null,
          rangeStart: "0",
          rangeEnd: "20",
          rangeStep: "",
          children: [
            {
              id: "2",
              name: "Line_",
              kind: "folder",
              dataType: null,
              rangeStart: "",
              rangeEnd: "4",
              rangeStep: "",
              children: [
                {
                  id: "3",
                  name: "Power",
                  kind: "variable",
                  dataType: "Float",
                  rangeStart: "",
                  rangeEnd: "",
                  rangeStep: "",
                  children: [],
                },
              ],
            },
          ],
        },
      ];
      const out = serializeYamlLike(nodes, 0, ALLOWED_TYPES);
      expect(out).toContain("Area_[0:20]:");
      expect(out).toContain("  Line_[:4]:");
      expect(out).toContain("    Power: Float");
    });

    it("round-trips with parseYamlLike", () => {
      const yaml =
        "Area_[0:20]:\n  Line_[:4]:\n    Motor_[:2]:\n      Power: Float\n      Speed: Float";
      resetId();
      const roots = parseYamlLike(yaml, {}, ALLOWED_TYPES, nextId);
      const serialized = serializeYamlLike(roots, 0, ALLOWED_TYPES);
      resetId();
      const reparsed = parseYamlLike(serialized, {}, ALLOWED_TYPES, nextId);
      expect(reparsed).toHaveLength(1);
      expect(reparsed[0].name).toBe("Area_");
      expect(reparsed[0].children[0].children[0].children).toHaveLength(2);
    });
  });

  describe("validateNamespaceAst", () => {
    it("does not throw for valid tree (all leaves have type)", () => {
      const nodes: NamespaceNode[] = [
        {
          id: "1",
          name: "F",
          kind: "folder",
          dataType: null,
          rangeStart: "",
          rangeEnd: "",
          rangeStep: "",
          children: [
            {
              id: "2",
              name: "X",
              kind: "variable",
              dataType: "Float",
              rangeStart: "",
              rangeEnd: "",
              rangeStep: "",
              children: [],
            },
          ],
        },
      ];
      expect(() => validateNamespaceAst(nodes, ALLOWED_TYPES)).not.toThrow();
    });

    it("throws for leaf folder without type", () => {
      const nodes: NamespaceNode[] = [
        {
          id: "1",
          name: "X",
          kind: "folder",
          dataType: null,
          rangeStart: "",
          rangeEnd: "",
          rangeStep: "",
          children: [],
        },
      ];
      expect(() => validateNamespaceAst(nodes, ALLOWED_TYPES)).toThrow(
        /leaf without type|must declare a data type/,
      );
    });
  });

  describe("normalizeLeafFoldersToVariables", () => {
    it("converts leaf folder to variable with default type", () => {
      const nodes: NamespaceNode[] = [
        {
          id: "1",
          name: "X",
          kind: "folder",
          dataType: null,
          rangeStart: "",
          rangeEnd: "",
          rangeStep: "",
          children: [],
        },
      ];
      normalizeLeafFoldersToVariables(nodes, ALLOWED_TYPES);
      expect(nodes[0].kind).toBe("variable");
      expect(nodes[0].dataType).toBe("Float");
    });

    it("leaves folders with children unchanged", () => {
      const nodes: NamespaceNode[] = [
        {
          id: "1",
          name: "F",
          kind: "folder",
          dataType: null,
          rangeStart: "",
          rangeEnd: "",
          rangeStep: "",
          children: [
            {
              id: "2",
              name: "X",
              kind: "variable",
              dataType: "Integer",
              rangeStart: "",
              rangeEnd: "",
              rangeStep: "",
              children: [],
            },
          ],
        },
      ];
      normalizeLeafFoldersToVariables(nodes, ALLOWED_TYPES);
      expect(nodes[0].kind).toBe("folder");
      expect(nodes[0].dataType).toBeNull();
    });
  });

  describe("normalizeFolderState", () => {
    it("sets kind to folder for nodes with children", () => {
      const node: NamespaceNode = {
        id: "1",
        name: "F",
        kind: "variable",
        dataType: "Float",
        rangeStart: "",
        rangeEnd: "",
        rangeStep: "",
        children: [
          {
            id: "2",
            name: "C",
            kind: "variable",
            dataType: "Float",
            rangeStart: "",
            rangeEnd: "",
            rangeStep: "",
            children: [],
          },
        ],
      };
      normalizeFolderState([node]);
      expect(node.kind).toBe("folder");
      expect(node.dataType).toBeNull();
    });
  });

  describe("astToNamespaceJson", () => {
    it("produces nested object with type strings at leaves", () => {
      const nodes: NamespaceNode[] = [
        {
          id: "1",
          name: "Area_",
          kind: "folder",
          dataType: null,
          rangeStart: "0",
          rangeEnd: "20",
          rangeStep: "",
          children: [
            {
              id: "2",
              name: "Line_",
              kind: "folder",
              dataType: null,
              rangeStart: "",
              rangeEnd: "4",
              rangeStep: "",
              children: [
                {
                  id: "3",
                  name: "Power",
                  kind: "variable",
                  dataType: "Float",
                  rangeStart: "",
                  rangeEnd: "",
                  rangeStep: "",
                  children: [],
                },
              ],
            },
          ],
        },
      ];
      const json = astToNamespaceJson(nodes, ALLOWED_TYPES);
      expect(json).toHaveProperty("Area_[0:20]");
      expect(
        (json["Area_[0:20]"] as Record<string, unknown>)["Line_[:4]"],
      ).toEqual({ Power: "Float" });
    });
  });

  describe("mergeTrailingBlankLinesAfterAutoFill", () => {
    it("pads after with blank lines when before has trailing blanks", () => {
      const before = "a:\n  b: Float\n\n";
      const after = "a:\n  b: Float";
      expect(mergeTrailingBlankLinesAfterAutoFill(before, after, 3)).toBe(
        "a:\n  b: Float\n\n",
      );
    });

    it("returns after when cursor not on blank line and same line count", () => {
      const before = "a:\n  b: Float";
      const after = "a:\n  b: Float";
      expect(mergeTrailingBlankLinesAfterAutoFill(before, after, 1)).toBe(
        after,
      );
    });
  });

  describe("buildAutoFillMergedYaml", () => {
    it("replaces folder-only lines with serialized line", () => {
      const before = "Power:\n";
      const after = "Power: Float";
      expect(buildAutoFillMergedYaml(before, after)).toBe("Power: Float\n");
    });

    it("keeps plain name lines from before", () => {
      const before = "abc\n";
      const after = "abc: Float";
      expect(buildAutoFillMergedYaml(before, after)).toBe("abc\n");
    });

    it("preserves trailing newline from before", () => {
      const before = "x:\n";
      const after = "x: Float";
      const merged = buildAutoFillMergedYaml(before, after);
      expect(merged.endsWith("\n")).toBe(true);
    });
  });
});
