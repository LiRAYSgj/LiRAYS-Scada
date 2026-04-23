export type ThemeMode = "light" | "dark";

export function createThemeVars(theme: ThemeMode): string {
  const isDark = theme === "dark";
  return `--bg-app:${isDark ? "#0F141A" : "#F5F7FA"};--bg-panel:${isDark ? "#161C23" : "#FFFFFF"};--bg-muted:${isDark ? "#1E2630" : "#EEF1F5"};--bg-hover:${isDark ? "#243041" : "#E6EBF2"};--bg-selected:${isDark ? "#1F3A6D" : "#DCE8FF"};--text-primary:${isDark ? "#E2E8F0" : "#0F172A"};--text-secondary:${isDark ? "#CBD5E1" : "#334155"};--text-muted:${isDark ? "#94A3B8" : "#64748B"};`;
}
