import { describe, expect, it } from "vitest";
import { createThemeVars } from "./theme-utils";

describe("createThemeVars", () => {
  it("returns light theme CSS variables when theme is light", () => {
    const vars = createThemeVars("light");
    expect(vars).toContain("--bg-app:#F5F7FA");
    expect(vars).toContain("--bg-panel:#FFFFFF");
    expect(vars).toContain("--text-primary:#0F172A");
    expect(vars).not.toContain("#0F141A");
  });

  it("returns dark theme CSS variables when theme is dark", () => {
    const vars = createThemeVars("dark");
    expect(vars).toContain("--bg-app:#0F141A");
    expect(vars).toContain("--bg-panel:#161C23");
    expect(vars).toContain("--text-primary:#E2E8F0");
    expect(vars).not.toContain("#F5F7FA");
  });

  it("includes all expected variable names", () => {
    const vars = createThemeVars("light");
    const names = [
      "--bg-app",
      "--bg-panel",
      "--bg-muted",
      "--bg-hover",
      "--bg-selected",
      "--text-primary",
      "--text-secondary",
      "--text-muted",
    ];
    for (const name of names) {
      expect(vars).toContain(name);
    }
  });
});
