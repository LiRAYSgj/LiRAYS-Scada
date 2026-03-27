import type {
  BoundWidgetTag,
  PlantAssetNodeData,
} from "$lib/features/graph/assets/types";
import type { TagScalarValue } from "$lib/core/ws/types";
import type {
  InternalWidgetDeclaration,
  WidgetManifest,
} from "$lib/scada/plugins/types";
import shellStyles from "./widget-shell.css?raw";

export interface WidgetViewModel {
  bodyHtml: string;
  footerLines?: string[];
}

export interface WidgetBindContext {
  root: ShadowRoot;
  data: PlantAssetNodeData;
  query<T extends Element>(selector: string): T | null;
  listen(target: EventTarget, eventName: string, handler: EventListener): void;
  debounce(key: string, delayMs: number, callback: () => void): void;
  getBindingValue(bindingKey: string): TagScalarValue | undefined;
  writeBinding(bindingKey: string, value: TagScalarValue): void;
}

export interface InternalCustomElementWidgetSpec {
  manifest: WidgetManifest;
  tagName: string;
  styles?: string;
  render(data: PlantAssetNodeData): WidgetViewModel;
  bind?(context: WidgetBindContext): void;
}

export function declareInternalCustomElementWidget(
  spec: InternalCustomElementWidgetSpec,
): InternalWidgetDeclaration {
  return {
    manifest: spec.manifest,
    runtime: {
      kind: "custom-element",
      tagName: spec.tagName,
      register: () => defineWidgetElement(spec),
    },
  };
}

function defineWidgetElement(spec: InternalCustomElementWidgetSpec): void {
  if (typeof window === "undefined") return;
  if (customElements.get(spec.tagName)) return;

  class ScadaInternalWidgetElement extends HTMLElement {
    private readonly root: ShadowRoot;
    private data: PlantAssetNodeData | null = null;
    private selected = false;
    private timers = new Map<string, ReturnType<typeof setTimeout>>();
    private cleanupCallbacks: Array<() => void> = [];

    private readonly onContextMenu = (event: Event): void => {
      if (!(event instanceof MouseEvent)) return;
      if (!this.data?.onOpenBindingConfig) return;
      event.preventDefault();
      event.stopPropagation();
      this.data.onOpenBindingConfig(event);
    };

    constructor() {
      super();
      this.root = this.attachShadow({ mode: "open" });
    }

    connectedCallback(): void {
      this.addEventListener("contextmenu", this.onContextMenu);
      this.render();
    }

    disconnectedCallback(): void {
      this.removeEventListener("contextmenu", this.onContextMenu);
      this.cleanupBindings();
      this.clearTimers();
    }

    set widgetData(value: PlantAssetNodeData) {
      this.data = value;
      this.render();
    }

    set widgetSelected(value: boolean) {
      this.selected = Boolean(value);
      this.render();
    }

    private render(): void {
      if (!this.data) return;

      this.setAttribute("data-selected", this.selected ? "true" : "false");
      this.cleanupBindings();

      const view = spec.render(this.data);
      this.root.innerHTML = renderWidgetShell({
        data: this.data,
        bodyHtml: view.bodyHtml,
        footerLines: view.footerLines,
        widgetStyles: spec.styles,
      });

      if (!spec.bind) return;
      spec.bind({
        root: this.root,
        data: this.data,
        query: <T extends Element>(selector: string): T | null =>
          this.root.querySelector<T>(selector),
        listen: (target: EventTarget, eventName: string, handler: EventListener): void => {
          target.addEventListener(eventName, handler);
          this.cleanupCallbacks.push(() => {
            target.removeEventListener(eventName, handler);
          });
        },
        debounce: (key: string, delayMs: number, callback: () => void): void => {
          const existing = this.timers.get(key);
          if (existing) clearTimeout(existing);
          const timer = setTimeout(() => {
            this.timers.delete(key);
            callback();
          }, delayMs);
          this.timers.set(key, timer);
        },
        getBindingValue: (bindingKey: string): TagScalarValue | undefined =>
          getBindingValue(this.data!, bindingKey),
        writeBinding: (bindingKey: string, value: TagScalarValue): void =>
          writeBinding(this.data!, bindingKey, value),
      });
    }

    private cleanupBindings(): void {
      for (const cleanup of this.cleanupCallbacks) cleanup();
      this.cleanupCallbacks = [];
    }

    private clearTimers(): void {
      for (const timer of this.timers.values()) clearTimeout(timer);
      this.timers.clear();
    }
  }

  customElements.define(spec.tagName, ScadaInternalWidgetElement);
}

interface RenderWidgetShellInput {
  data: PlantAssetNodeData;
  bodyHtml: string;
  footerLines?: string[];
  widgetStyles?: string;
}

function renderWidgetShell({
  data,
  bodyHtml,
  footerLines,
  widgetStyles,
}: RenderWidgetShellInput): string {
  const primary = getPrimaryTag(data);
  const title = escapeHtml(data.title ?? "Widget");
  const kind = escapeHtml(String(data.assetKind ?? "widget"));
  const sourceName = escapeHtml(primary?.name ?? "Unbound");
  const sourcePath = escapeHtml(primary?.path ?? "-");
  const lines = [
    `Source: ${sourceName}`,
    sourcePath,
    ...(footerLines ?? []),
  ]
    .map((line) => `<div>${escapeHtml(line)}</div>`)
    .join("");

  return `
<style>${shellStyles}${widgetStyles ?? ""}</style>
<article class="wrap">
  <header class="head">
    <strong class="title">${title}</strong>
    <span class="kind">${kind}</span>
  </header>
  <section class="body">${bodyHtml}</section>
  <footer class="foot">${lines}</footer>
</article>
`;
}

export function toNumeric(value: TagScalarValue | undefined): number | null {
  if (typeof value === "number" && Number.isFinite(value)) return value;
  if (typeof value === "boolean") return value ? 100 : 0;
  if (typeof value === "string") {
    const parsed = Number(value);
    if (Number.isFinite(parsed)) return parsed;
  }
  return null;
}

export function toPercent(value: TagScalarValue | undefined): number {
  const numeric = toNumeric(value);
  if (numeric === null) return 0;
  const scaled = Math.abs(numeric) <= 1 ? numeric * 100 : numeric;
  return Math.max(0, Math.min(100, Math.round(scaled)));
}

export function toBoolean(value: TagScalarValue | undefined): boolean {
  if (typeof value === "boolean") return value;
  if (typeof value === "number") return value >= 50;
  if (typeof value === "string") {
    const normalized = value.trim().toLowerCase();
    return (
      normalized === "true" ||
      normalized === "1" ||
      normalized === "on" ||
      normalized === "yes"
    );
  }
  return false;
}

export function toText(value: TagScalarValue | undefined): string {
  return value === undefined ? "--" : String(value);
}

export function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

export function renderTemplate(
  template: string,
  values: Record<string, string | number | boolean | null | undefined>,
): string {
  return template.replaceAll(/\{\{\s*([a-zA-Z0-9_]+)\s*\}\}/g, (_, key: string) => {
    const value = values[key];
    if (value === null || value === undefined) return "";
    return escapeHtml(String(value));
  });
}

export function getPrimaryTag(data: PlantAssetNodeData): BoundWidgetTag | undefined {
  if (data.bindings && data.primaryBindingKey) {
    return data.bindings[data.primaryBindingKey]?.[0];
  }
  return data.sourceNode;
}

export function getBindingValue(
  data: PlantAssetNodeData,
  key: string,
): TagScalarValue | undefined {
  const value = data.liveValues?.[key];
  if (Array.isArray(value)) return value[0];
  return value as TagScalarValue | undefined;
}

export function writeBinding(
  data: PlantAssetNodeData,
  bindingKey: string,
  value: TagScalarValue,
): void {
  const tagId = data.bindings?.[bindingKey]?.[0]?.id;
  if (data.onWriteBindingValue) {
    data.onWriteBindingValue(bindingKey, value, tagId);
    return;
  }
  data.onWriteValue?.(value);
}
