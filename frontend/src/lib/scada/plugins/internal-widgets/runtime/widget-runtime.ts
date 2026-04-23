import type {
  BoundWidgetTag,
  PlantAssetNodeData,
  WidgetInteractionEventName,
} from "$lib/features/graph/assets/types";
import type { TagScalarValue } from "$lib/core/ws/types";
import {
  getWidgetHandlers,
  type WidgetRuntimeHandlers,
} from "$lib/features/graph/widget-handlers";
import type {
  InternalWidgetDeclaration,
  WidgetManifest,
} from "$lib/scada/plugins/types";
import Mustache from "mustache";
import shellStyles from "./widget-shell.css?raw";

export interface WidgetViewModel {
  bodyHtml: string;
  footerLines?: string[];
}

export interface NumericRange {
  min: number;
  max: number;
}

export interface WidgetBindContext {
  root: ShadowRoot;
  data: PlantAssetNodeData;
  query<T extends Element>(selector: string): T | null;
  listen(target: EventTarget, eventName: string, handler: EventListener): void;
  debounce(key: string, delayMs: number, callback: () => void): void;
  getBindingValue(bindingKey: string): TagScalarValue | undefined;
  writeBinding(bindingKey: string, value: TagScalarValue): void;
  events: {
    emit(name: WidgetInteractionEventName, payload?: unknown): void;
  };
}

export interface InternalCustomElementWidgetSpec {
  manifest: WidgetManifest;
  tagName: string;
  styles?: string;
  interactionSourceSelector?: string;
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
    private resizeObserver: ResizeObserver | null = null;
    private autoscaleRaf: number | null = null;
    private readonly interactionSourceSelector = spec.interactionSourceSelector;
    private readonly isInputWidget = Boolean(
      spec.manifest.capabilities?.inputWidget,
    );

    private isEventFromAllowedSource(event: Event): boolean {
      const selector = this.interactionSourceSelector;
      if (!selector) return true;
      const path = event.composedPath();
      return path.some(
        (entry) => entry instanceof Element && entry.matches(selector),
      );
    }

    private readonly onContextMenu = (event: Event): void => {
      if (!(event instanceof MouseEvent)) return;
      if (!this.data) return;
      if (!this.isEventFromAllowedSource(event)) return;
      const handlers = resolveWidgetHandlers(this.data);

      if (
        this.data.interactionMode === "editor" &&
        handlers.onOpenBindingConfig
      ) {
        event.preventDefault();
        event.stopPropagation();
        handlers.onOpenBindingConfig(event);
        return;
      }

      if (this.data.interactionMode === "runtime" && handlers.onWidgetEvent) {
        event.preventDefault();
        event.stopPropagation();
        handlers.onWidgetEvent("rightClick", undefined, event);
      }
    };

    private readonly onClick = (event: Event): void => {
      if (!(event instanceof MouseEvent)) return;
      if (this.data?.interactionMode !== "runtime") return;
      if (!this.isEventFromAllowedSource(event)) return;
      resolveWidgetHandlers(this.data).onWidgetEvent?.(
        "click",
        undefined,
        event,
      );
    };

    private readonly onDoubleClick = (event: Event): void => {
      if (!(event instanceof MouseEvent)) return;
      if (this.data?.interactionMode !== "runtime") return;
      if (!this.isEventFromAllowedSource(event)) return;
      resolveWidgetHandlers(this.data).onWidgetEvent?.(
        "doubleClick",
        undefined,
        event,
      );
    };

    constructor() {
      super();
      this.root = this.attachShadow({ mode: "open" });
    }

    connectedCallback(): void {
      this.addEventListener("contextmenu", this.onContextMenu);
      this.addEventListener("click", this.onClick);
      this.addEventListener("dblclick", this.onDoubleClick);
      this.ensureResizeObserver();
      this.updateSizeCssVars();
      this.render();
    }

    disconnectedCallback(): void {
      this.removeEventListener("contextmenu", this.onContextMenu);
      this.removeEventListener("click", this.onClick);
      this.removeEventListener("dblclick", this.onDoubleClick);
      this.resizeObserver?.disconnect();
      this.resizeObserver = null;
      if (this.autoscaleRaf !== null) {
        cancelAnimationFrame(this.autoscaleRaf);
        this.autoscaleRaf = null;
      }
      this.cleanupBindings();
      this.clearTimers();
    }

    set widgetData(value: PlantAssetNodeData) {
      if (this.data === value) return;
      this.data = value;
      this.render();
    }

    set widgetSelected(value: boolean) {
      const next = Boolean(value);
      if (this.selected === next) return;
      this.selected = next;
      this.render();
    }

    private render(): void {
      if (!this.data) return;

      this.setAttribute("data-selected", this.selected ? "true" : "false");
      this.setAttribute(
        "data-input-widget",
        this.isInputWidget ? "true" : "false",
      );
      this.cleanupBindings();

      const view = spec.render(this.data);
      this.root.innerHTML = renderWidgetShell({
        data: this.data,
        bodyHtml: view.bodyHtml,
        widgetStyles: spec.styles,
        showTitle: spec.manifest.capabilities?.showTitle !== false,
        overlayTitle: Boolean(spec.manifest.capabilities?.titleOverlaysBody),
      });
      this.scheduleAutoscaleText();

      if (!spec.bind) return;
      spec.bind({
        root: this.root,
        data: this.data,
        query: <T extends Element>(selector: string): T | null =>
          this.root.querySelector<T>(selector),
        listen: (
          target: EventTarget,
          eventName: string,
          handler: EventListener,
        ): void => {
          target.addEventListener(eventName, handler);
          this.cleanupCallbacks.push(() => {
            target.removeEventListener(eventName, handler);
          });
        },
        debounce: (
          key: string,
          delayMs: number,
          callback: () => void,
        ): void => {
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
        events: {
          emit: (name: WidgetInteractionEventName, payload?: unknown): void => {
            if (this.data?.interactionMode !== "runtime") return;
            resolveWidgetHandlers(this.data).onWidgetEvent?.(name, payload);
          },
        },
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

    private ensureResizeObserver(): void {
      if (this.resizeObserver || typeof ResizeObserver === "undefined") return;
      this.resizeObserver = new ResizeObserver(() => {
        this.updateSizeCssVars();
        this.scheduleAutoscaleText();
      });
      this.resizeObserver.observe(this);
    }

    private updateSizeCssVars(): void {
      const width = Math.max(0, this.clientWidth);
      const height = Math.max(0, this.clientHeight);
      const minSize = Math.min(width, height);
      this.style.setProperty("--widget-width", `${width}px`);
      this.style.setProperty("--widget-height", `${height}px`);
      this.style.setProperty("--widget-min-size", `${minSize}px`);
    }

    private scheduleAutoscaleText(): void {
      if (this.autoscaleRaf !== null) {
        cancelAnimationFrame(this.autoscaleRaf);
      }
      this.autoscaleRaf = requestAnimationFrame(() => {
        this.autoscaleRaf = null;
        this.autoscaleTextToFit();
      });
    }

    private autoscaleTextToFit(): void {
      const elements = this.root.querySelectorAll<HTMLElement>(
        '[data-autoscale-text="true"]',
      );
      for (const element of elements) {
        const container = element.parentElement as HTMLElement | null;
        if (!container) continue;
        const availableWidth = Math.max(0, container.clientWidth);
        const availableHeight = Math.max(0, container.clientHeight);
        if (availableWidth === 0 || availableHeight === 0) continue;

        const minSize = 12;
        const maxSize = Math.max(
          minSize,
          Math.min(
            220,
            Math.floor(Math.min(availableWidth, availableHeight) * 0.9),
          ),
        );

        let low = minSize;
        let high = maxSize;
        let best = minSize;

        while (low <= high) {
          const mid = Math.floor((low + high) / 2);
          element.style.fontSize = `${mid}px`;
          const fits =
            element.scrollWidth <= availableWidth &&
            element.scrollHeight <= availableHeight;
          if (fits) {
            best = mid;
            low = mid + 1;
          } else {
            high = mid - 1;
          }
        }

        element.style.fontSize = `${best}px`;
      }
    }
  }

  customElements.define(spec.tagName, ScadaInternalWidgetElement);
}

interface RenderWidgetShellInput {
  data: PlantAssetNodeData;
  bodyHtml: string;
  widgetStyles?: string;
  showTitle?: boolean;
  overlayTitle?: boolean;
}

function renderWidgetShell({
  data,
  bodyHtml,
  widgetStyles,
  showTitle = true,
  overlayTitle = false,
}: RenderWidgetShellInput): string {
  const title = escapeHtml(data.title ?? "Widget");
  const wrapClasses = ["wrap"];
  if (!showTitle) wrapClasses.push("no-head");
  if (showTitle && overlayTitle) wrapClasses.push("overlay-head");

  return `
<style>${shellStyles}${widgetStyles ?? ""}</style>
<article class="${wrapClasses.join(" ")}">
  ${
    showTitle
      ? `<header class="head">
    <strong class="title">${title}</strong>
  </header>
`
      : ""
  }
  <section class="body">${bodyHtml}</section>
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
  return toPercentInRange(value, resolveNumericRange());
}

export function resolveNumericRange(min?: number, max?: number): NumericRange {
  const hasMin = typeof min === "number" && Number.isFinite(min);
  const hasMax = typeof max === "number" && Number.isFinite(max);

  if (hasMin && hasMax) {
    if (min === max) {
      return { min, max: max + 100 };
    }
    return min < max ? { min, max } : { min: max, max: min };
  }

  if (hasMin) {
    return { min, max: min + 100 };
  }

  if (hasMax) {
    return { min: max - 100, max };
  }

  return { min: 0, max: 100 };
}

export function toPercentInRange(
  value: TagScalarValue | undefined,
  range: NumericRange,
): number {
  const numeric = toNumeric(value);
  if (numeric === null) return 0;
  const span = range.max - range.min;
  if (!Number.isFinite(span) || span === 0) return 0;
  return Math.max(
    0,
    Math.min(100, Math.round(((numeric - range.min) / span) * 100)),
  );
}

export function toBoolean(value: TagScalarValue | undefined): boolean {
  if (typeof value === "boolean") return value;
  if (typeof value === "number") return Number.isFinite(value) && value !== 0;
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
  return renderMustacheTemplate(template, values);
}

export function renderMustacheTemplate(
  template: string,
  view: Record<string, unknown>,
): string {
  const previousEscape = Mustache.escape;
  Mustache.escape = escapeHtml;
  try {
    return Mustache.render(template, view);
  } finally {
    Mustache.escape = previousEscape;
  }
}

export function getPrimaryTag(
  data: PlantAssetNodeData,
): BoundWidgetTag | undefined {
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

export function getBindingNumericRange(
  data: PlantAssetNodeData,
  key: string,
): NumericRange {
  const tag = getBindingTag(data, key);
  return resolveNumericRange(tag?.min, tag?.max);
}

export function getBindingUnit(data: PlantAssetNodeData, key: string): string {
  const unit = getBindingTag(data, key)?.unit;
  return typeof unit === "string" ? unit : "";
}

export function getBindingNumericRangeInferred(
  data: PlantAssetNodeData,
  key: string,
  currentValue?: TagScalarValue,
): NumericRange {
  const tag = getBindingTag(data, key);
  const hasExplicitRange =
    typeof tag?.min === "number" || typeof tag?.max === "number";
  if (hasExplicitRange) {
    return resolveNumericRange(tag?.min, tag?.max);
  }

  const numeric = toNumeric(currentValue);
  if (numeric === null) {
    return resolveNumericRange();
  }

  return resolveNumericRange(numeric - 50, numeric + 50);
}

export function getWidgetConfigValue<T>(
  data: PlantAssetNodeData,
  key: string,
  fallback: T,
): T {
  const value = data.widgetConfig?.[key];
  return value === undefined ? fallback : (value as T);
}

export function writeBinding(
  data: PlantAssetNodeData,
  bindingKey: string,
  value: TagScalarValue,
): void {
  if (data.interactionMode !== "runtime") {
    return;
  }
  const tagId = data.bindings?.[bindingKey]?.[0]?.id;
  const handlers = resolveWidgetHandlers(data);
  if (handlers.onWriteBindingValue) {
    handlers.onWriteBindingValue(bindingKey, value, tagId);
    return;
  }
  handlers.onWriteValue?.(value);
}

function resolveWidgetHandlers(
  data: PlantAssetNodeData,
): WidgetRuntimeHandlers {
  const handlers = getWidgetHandlers(data.symbolId);
  return {
    onWriteValue: handlers?.onWriteValue ?? data.onWriteValue,
    onWriteBindingValue:
      handlers?.onWriteBindingValue ?? data.onWriteBindingValue,
    onOpenBindingConfig:
      handlers?.onOpenBindingConfig ?? data.onOpenBindingConfig,
    onWidgetEvent: handlers?.onWidgetEvent ?? data.onWidgetEvent,
  };
}

function getBindingTag(
  data: PlantAssetNodeData,
  key: string,
): BoundWidgetTag | undefined {
  return (
    data.bindings?.[key]?.find((binding) => binding.kind === "tag") ??
    (data.primaryBindingKey === key && data.sourceNode?.kind === "tag"
      ? data.sourceNode
      : undefined)
  );
}
