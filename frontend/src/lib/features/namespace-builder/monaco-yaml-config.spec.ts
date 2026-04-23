import { describe, expect, it } from "vitest";
import {
  UNS_YAML_LANGUAGE_ID,
  UNS_YAML_THEME_DARK_ID,
  UNS_YAML_THEME_LIGHT_ID,
  getYamlLiteTokenizer,
  getUnsYamlDarkTheme,
  getUnsYamlLightTheme,
  getMonacoThemeId,
} from "./monaco-yaml-config.js";

describe("monaco-yaml-config", () => {
  describe("constants", () => {
    it("exports language and theme ids", () => {
      expect(UNS_YAML_LANGUAGE_ID).toBe("uns-yaml-lite");
      expect(UNS_YAML_THEME_DARK_ID).toBe("uns-yaml-dark");
      expect(UNS_YAML_THEME_LIGHT_ID).toBe("uns-yaml-light");
    });
  });

  describe("getMonacoThemeId", () => {
    it('returns dark theme id for colorMode "dark"', () => {
      expect(getMonacoThemeId("dark")).toBe(UNS_YAML_THEME_DARK_ID);
    });
    it('returns light theme id for colorMode "light"', () => {
      expect(getMonacoThemeId("light")).toBe(UNS_YAML_THEME_LIGHT_ID);
    });
  });

  describe("getYamlLiteTokenizer", () => {
    it("returns object with tokenizer.root array", () => {
      const tokenizer = getYamlLiteTokenizer();
      expect(tokenizer).toHaveProperty("tokenizer");
      expect(tokenizer.tokenizer).toHaveProperty("root");
      expect(Array.isArray(tokenizer.tokenizer.root)).toBe(true);
      expect(tokenizer.tokenizer.root.length).toBeGreaterThan(0);
    });
    it("root has entries for variable line, folder line, and bracket", () => {
      const tokenizer = getYamlLiteTokenizer();
      expect(tokenizer.tokenizer.root.length).toBe(3);
    });
  });

  describe("getUnsYamlDarkTheme", () => {
    it("returns theme with base vs-dark", () => {
      const theme = getUnsYamlDarkTheme();
      expect(theme.base).toBe("vs-dark");
      expect(theme.inherit).toBe(true);
    });
    it("includes rules for folder, variable, type, delimiter, bracket", () => {
      const theme = getUnsYamlDarkTheme();
      const tokens = theme.rules?.map((r) => r.token) ?? [];
      expect(tokens).toContain("folder");
      expect(tokens).toContain("variable");
      expect(tokens).toContain("type");
      expect(tokens).toContain("delimiter");
      expect(tokens).toContain("bracket");
    });
    it("includes editor.background and editor.foreground colors", () => {
      const theme = getUnsYamlDarkTheme();
      expect(theme.colors).toHaveProperty("editor.background");
      expect(theme.colors).toHaveProperty("editor.foreground");
    });
  });

  describe("getUnsYamlLightTheme", () => {
    it("returns theme with base vs", () => {
      const theme = getUnsYamlLightTheme();
      expect(theme.base).toBe("vs");
      expect(theme.inherit).toBe(true);
    });
    it("includes same token rules as dark", () => {
      const theme = getUnsYamlLightTheme();
      const tokens = theme.rules?.map((r) => r.token) ?? [];
      expect(tokens).toContain("folder");
      expect(tokens).toContain("variable");
      expect(tokens).toContain("type");
    });
    it("has light background color", () => {
      const theme = getUnsYamlLightTheme();
      expect(theme.colors?.["editor.background"]).toBe("#FFFFFF");
      expect(theme.colors?.["editor.foreground"]).toBe("#24292F");
    });
  });
});
