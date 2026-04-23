import { marked } from "marked";
import { PlantAssetKind } from "$lib/features/graph/assets/types";
import {
  declareInternalCustomElementWidget,
  escapeHtml,
  getWidgetConfigValue,
} from "$lib/scada/plugins/internal-widgets/runtime/widget-runtime";
import styles from "./markdown.widget.css?raw";

const DEFAULT_CONTENT = "# Markdown\nWrite *formatted* text here.";

function sanitizeHref(rawHref: string): string {
  const trimmed = rawHref.trim();
  if (!trimmed) {
    return "#";
  }
  if (trimmed.startsWith("/") || trimmed.startsWith("#")) {
    return escapeHtml(trimmed);
  }
  try {
    const parsed = new URL(trimmed);
    if (
      parsed.protocol === "http:" ||
      parsed.protocol === "https:" ||
      parsed.protocol === "mailto:"
    ) {
      return escapeHtml(trimmed);
    }
  } catch {
    return "#";
  }
  return "#";
}

const renderer = new marked.Renderer();

renderer.link = ({ href, title, text }) => {
  const safeHref = sanitizeHref(href ?? "");
  const titleAttr = title ? ` title="${escapeHtml(title)}"` : "";
  return `<a href="${safeHref}"${titleAttr} target="_blank" rel="noreferrer">${text}</a>`;
};

renderer.html = ({ text }) => escapeHtml(text ?? "");

function renderMarkdown(content: string): string {
  const trimmed = content.trim();
  if (!trimmed) {
    return '<p class="markdown-empty">No content</p>';
  }

  return marked.parse(escapeHtml(content), {
    gfm: true,
    breaks: true,
    renderer,
  }) as string;
}

export const markdownWidget = declareInternalCustomElementWidget({
  manifest: {
    type: PlantAssetKind.MARKDOWN,
    displayName: "Markdown",
    version: "1.0.0",
    apiVersion: "1",
    primaryBindingKey: "",
    bindings: [],
    configSchema: [
      {
        key: "content",
        label: "Content",
        type: "textarea",
        placeholder: "# Title\\nWrite markdown content...",
      },
    ],
    defaultConfig: {
      content: DEFAULT_CONTENT,
    },
    capabilities: {
      keepAspectRatio: false,
      minWidth: 180,
      minHeight: 120,
    },
  },
  tagName: "lirays-widget-markdown",
  styles,
  render: (data) => {
    const content = String(
      getWidgetConfigValue(data, "content", DEFAULT_CONTENT),
    );
    return {
      bodyHtml: `<div class="panel markdown-panel">${renderMarkdown(content)}</div>`,
    };
  },
});
